use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, Value, types};
use cranelift_frontend::FunctionBuilder;

use crate::ast::{Argument, IntrinsicKind, lookup_call_intrinsic};

use super::super::layout::LayoutRegistry;
use super::super::native::{FunctionRef, NativeEmitError};
use super::super::types::NativeType;
use super::arguments::lower_call_arguments;
use super::context::CallLoweringContext;
use super::returns::allocate_return_address;

pub(crate) fn lower_call(
    function: &mut FunctionBuilder<'_>,
    callee: &str,
    arguments: &[Argument],
    context: &CallLoweringContext<'_, '_>,
) -> Result<Value, NativeEmitError> {
    let callee_name = callee.split_once('[').map_or(callee, |(name, _)| name);
    let target = call_target(context.functions, callee_name)?;
    let mut values = lower_call_arguments(
        function,
        arguments,
        &target.parameter_names,
        &target.dynamic_params,
        &target.dynamic_roles,
        &target.ins_params,
        context,
    )?;
    let target = resolve_call_target(function, callee_name, arguments, target, &values, context)?;
    normalize_call_arguments(function, callee_name, &mut values)?;
    let result_address = allocate_return_address(function, target.return_type, context.layouts)?;
    if let Some(address) = result_address {
        values.insert(0, address);
    }
    let call = function.ins().call(target.reference, &values);
    finish_call(function, call, result_address, target.return_type, callee_name, context.layouts)
}

fn call_target<'a>(
    functions: &'a HashMap<String, FunctionRef>,
    callee: &str,
) -> Result<&'a FunctionRef, NativeEmitError> {
    functions
        .get(callee)
        .ok_or_else(|| NativeEmitError(format!("native function `{callee}` is unavailable")))
}

fn finish_call(
    function: &mut FunctionBuilder<'_>,
    call: cranelift_codegen::ir::Inst,
    result_address: Option<Value>,
    return_type: NativeType,
    callee: &str,
    layouts: &LayoutRegistry,
) -> Result<Value, NativeEmitError> {
    if let Some(address) = result_address {
        if layouts.returns_borrowed_view(return_type) {
            return function.inst_results(call).first().copied().ok_or_else(|| {
                NativeEmitError("borrowed view call returned no pointer".to_owned())
            });
        }
        return Ok(address);
    }
    if matches!(return_type, NativeType::Void) {
        return Ok(function.ins().iconst(types::I32, 0));
    }
    function
        .inst_results(call)
        .first()
        .copied()
        .ok_or_else(|| NativeEmitError(format!("native function `{callee}` returned no value")))
}

fn resolve_call_target<'a>(
    function: &FunctionBuilder<'_>,
    callee: &str,
    arguments: &[Argument],
    target: &'a FunctionRef,
    values: &[Value],
    context: &'a CallLoweringContext<'a, '_>,
) -> Result<&'a FunctionRef, NativeEmitError> {
    if lookup_call_intrinsic(callee) == Some(IntrinsicKind::Print)
        && target.parameter_names == ["value"]
        && values.first().is_some_and(|value| function.func.dfg.value_type(*value) == types::I64)
    {
        let argument_type = arguments.first().and_then(|argument| {
            super::super::expressions::initializer_type(
                &argument.expression,
                context.local_types,
                context.functions,
                context.layouts,
            )
            .ok()
        });
        let symbol = if argument_type == Some(NativeType::Buffer) {
            "actus_print_buffer_stdout"
        } else {
            "actus_print_string"
        };
        context.functions.get(symbol).ok_or_else(|| {
            NativeEmitError(format!("native runtime function `{symbol}` is unavailable"))
        })
    } else {
        Ok(target)
    }
}

fn normalize_call_arguments(
    function: &mut FunctionBuilder<'_>,
    callee: &str,
    values: &mut Vec<Value>,
) -> Result<(), NativeEmitError> {
    if let Some(IntrinsicKind::Append) = lookup_call_intrinsic(callee) {
        let byte =
            values.pop().ok_or_else(|| NativeEmitError("append requires a byte".to_owned()))?;
        values.push(to_byte(function, byte));
    }
    Ok(())
}

fn to_byte(function: &mut FunctionBuilder<'_>, value: Value) -> Value {
    if function.func.dfg.value_type(value) == types::I8 {
        value
    } else {
        function.ins().ireduce(types::I8, value)
    }
}
