use cranelift_codegen::ir::{GlobalValue, InstBuilder, Value};
use cranelift_frontend::FunctionBuilder;

use crate::ast::{Argument, Expr};

use super::super::expressions::lower_expression;
use super::super::native::NativeEmitError;
use super::context::CallLoweringContext;

pub(super) fn lower_call_arguments(
    function: &mut FunctionBuilder<'_>,
    arguments: &[Argument],
    parameter_names: &[String],
    dynamic_params: &[bool],
    dynamic_roles: &[Option<String>],
    context: &CallLoweringContext<'_, '_>,
) -> Result<Vec<Value>, NativeEmitError> {
    let ordered = order_arguments(arguments, parameter_names)?;
    let mut values = Vec::new();
    for (index, argument) in ordered.iter().enumerate() {
        lower_one_call_argument(
            function,
            argument,
            index,
            dynamic_params,
            dynamic_roles,
            context,
            &mut values,
        )?;
    }
    Ok(values)
}

fn lower_one_call_argument(
    function: &mut FunctionBuilder<'_>,
    argument: &Expr,
    index: usize,
    dynamic_params: &[bool],
    dynamic_roles: &[Option<String>],
    context: &CallLoweringContext<'_, '_>,
    values: &mut Vec<Value>,
) -> Result<(), NativeEmitError> {
    let value = lower_expression(
        function,
        argument,
        context.locals,
        context.local_types,
        context.functions,
        context.cleanup_schedule,
        context.string_data,
        context.layouts,
    )?;
    values.push(value);
    append_dynamic_argument(
        function,
        argument,
        index,
        dynamic_params,
        dynamic_roles,
        context,
        values,
    )
}

fn append_dynamic_argument(
    function: &mut FunctionBuilder<'_>,
    argument: &Expr,
    index: usize,
    dynamic_params: &[bool],
    dynamic_roles: &[Option<String>],
    context: &CallLoweringContext<'_, '_>,
    values: &mut Vec<Value>,
) -> Result<(), NativeEmitError> {
    if !is_dynamic_parameter(dynamic_params, index)? {
        return Ok(());
    }
    let role = dynamic_roles
        .get(index)
        .and_then(Option::as_deref)
        .ok_or_else(|| NativeEmitError("dynamic parameter has no role".to_owned()))?;
    let global = dynamic_vtable_value(argument, role, context)?;
    values.push(function.ins().symbol_value(context.layouts.pointer_type, global));
    Ok(())
}

fn dynamic_vtable_value(
    argument: &Expr,
    role: &str,
    context: &CallLoweringContext<'_, '_>,
) -> Result<GlobalValue, NativeEmitError> {
    let native_type = super::super::expressions::initializer_type(
        argument,
        context.local_types,
        context.functions,
        context.layouts,
    )?;
    let symbol = super::super::vtable::vtable_symbol_for_native(role, native_type);
    context
        .string_data
        .get(&symbol)
        .copied()
        .ok_or_else(|| NativeEmitError(format!("vtable `{symbol}` has no native data")))
}

fn is_dynamic_parameter(dynamic_params: &[bool], index: usize) -> Result<bool, NativeEmitError> {
    dynamic_params
        .get(index)
        .copied()
        .ok_or_else(|| NativeEmitError("call metadata is missing parameter mode".to_owned()))
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
