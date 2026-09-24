use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, types};
use cranelift_frontend::FunctionBuilder;

use crate::ast::{Argument, Expr, IntrinsicKind, lookup_call_intrinsic};

use super::expressions::lower_expression;
use super::layout::LayoutRegistry;
use super::literals::StringDataValues;
use super::model::NativeCleanupSchedule;
use super::native::{FunctionRef, NativeEmitError};
use super::performance::dispatch_key;
use super::types::NativeType;

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_method_call(
    function: &mut FunctionBuilder<'_>,
    receiver: &Expr,
    method: &str,
    arguments: &[Argument],
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let named = arguments.iter().any(|argument| argument.name.is_some());
    let mut combined = Vec::with_capacity(arguments.len() + 1);
    combined
        .push(Argument { name: named.then(|| "self".to_owned()), expression: receiver.clone() });
    combined.extend(arguments.iter().cloned());
    let receiver_type =
        super::expressions::initializer_type(receiver, local_types, functions, layouts);
    let dispatch_name = dispatch_key(receiver_type, method);
    let callee =
        if functions.contains_key(&dispatch_name) { dispatch_name.as_str() } else { method };
    lower_call(
        function,
        callee,
        &combined,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_call(
    function: &mut FunctionBuilder<'_>,
    callee: &str,
    arguments: &[Argument],
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let target = functions
        .get(callee)
        .ok_or_else(|| NativeEmitError(format!("native function `{callee}` is unavailable")))?;
    let mut values = lower_call_arguments(
        function,
        arguments,
        &target.parameter_names,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    let target = resolve_call_target(function, callee, target, &values, functions)?;
    normalize_call_arguments(function, callee, &mut values)?;
    let call = function.ins().call(target.reference, &values);
    function
        .inst_results(call)
        .first()
        .copied()
        .ok_or_else(|| NativeEmitError(format!("native function `{callee}` returned no value")))
}

#[allow(clippy::too_many_arguments)]
fn lower_call_arguments(
    function: &mut FunctionBuilder<'_>,
    arguments: &[Argument],
    parameter_names: &[String],
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Vec<cranelift_codegen::ir::Value>, NativeEmitError> {
    order_arguments(arguments, parameter_names)?
        .iter()
        .map(|argument| {
            lower_expression(
                function,
                argument,
                locals,
                local_types,
                functions,
                cleanup_schedule,
                string_data,
                layouts,
            )
        })
        .collect()
}

fn resolve_call_target<'a>(
    function: &FunctionBuilder<'_>,
    callee: &str,
    target: &'a FunctionRef,
    values: &[cranelift_codegen::ir::Value],
    functions: &'a HashMap<String, FunctionRef>,
) -> Result<&'a FunctionRef, NativeEmitError> {
    if lookup_call_intrinsic(callee) == Some(IntrinsicKind::Print)
        && values.first().is_some_and(|value| function.func.dfg.value_type(*value) == types::I64)
    {
        functions.get("actus_print_string").ok_or_else(|| {
            NativeEmitError(
                "native runtime function `actus_print_string` is unavailable".to_owned(),
            )
        })
    } else {
        Ok(target)
    }
}

fn normalize_call_arguments(
    function: &mut FunctionBuilder<'_>,
    callee: &str,
    values: &mut Vec<cranelift_codegen::ir::Value>,
) -> Result<(), NativeEmitError> {
    match lookup_call_intrinsic(callee) {
        Some(IntrinsicKind::Allocate) => {
            let length = values
                .pop()
                .ok_or_else(|| NativeEmitError("allocate requires a length".to_owned()))?;
            values.push(to_pointer_length(function, length));
        }
        Some(IntrinsicKind::Append) => {
            let byte =
                values.pop().ok_or_else(|| NativeEmitError("append requires a byte".to_owned()))?;
            values.push(to_byte(function, byte));
        }
        _ => {}
    }
    Ok(())
}

fn to_pointer_length(
    function: &mut FunctionBuilder<'_>,
    value: cranelift_codegen::ir::Value,
) -> cranelift_codegen::ir::Value {
    if function.func.dfg.value_type(value) == types::I64 {
        value
    } else {
        function.ins().uextend(types::I64, value)
    }
}

fn to_byte(
    function: &mut FunctionBuilder<'_>,
    value: cranelift_codegen::ir::Value,
) -> cranelift_codegen::ir::Value {
    if function.func.dfg.value_type(value) == types::I8 {
        value
    } else {
        function.ins().ireduce(types::I8, value)
    }
}

fn order_arguments<'source>(
    arguments: &'source [Argument],
    parameter_names: &[String],
) -> Result<Vec<&'source Expr>, NativeEmitError> {
    if arguments.iter().all(|argument| argument.name.is_none()) {
        return Ok(arguments.iter().map(|argument| &argument.expression).collect());
    }
    if arguments.iter().any(|argument| argument.name.is_none()) {
        return Err(NativeEmitError(
            "native calls cannot mix named and positional arguments".to_owned(),
        ));
    }
    parameter_names
        .iter()
        .map(|parameter| {
            arguments
                .iter()
                .find(|argument| argument.name.as_deref() == Some(parameter))
                .map(|argument| &argument.expression)
                .ok_or_else(|| NativeEmitError(format!("missing native argument `{parameter}")))
        })
        .collect()
}
