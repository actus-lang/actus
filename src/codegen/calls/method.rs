use cranelift_codegen::ir::Value;
use cranelift_frontend::FunctionBuilder;

use crate::ast::{Argument, ArgumentRoleResolution, Expr};

use super::super::dynamic_call::lower_dynamic_call;
use super::super::expressions::lower_expression;
use super::super::native::NativeEmitError;
use super::super::performance::dispatch_key;
use super::super::types::NativeType;
use super::context::CallLoweringContext;
use super::lower::lower_call;

pub(crate) fn lower_method_call(
    function: &mut FunctionBuilder<'_>,
    receiver: &Expr,
    method: &str,
    arguments: &[Argument],
    context: &CallLoweringContext<'_, '_>,
) -> Result<Value, NativeEmitError> {
    let combined = combine_method_arguments(receiver, arguments);
    let receiver_type = receiver_native_type(receiver, context)?;
    if receiver_type == NativeType::FatPointer {
        return lower_fat_pointer_method(function, receiver, context);
    }
    let callee = method_callee(receiver_type, method, context.functions);
    lower_call(function, &callee, &combined, context)
}

fn combine_method_arguments(receiver: &Expr, arguments: &[Argument]) -> Vec<Argument> {
    let named = arguments.iter().any(|argument| argument.name.is_some());
    let mut combined = Vec::with_capacity(arguments.len() + 1);
    combined.push(Argument {
        name: named.then(|| "self".to_owned()),
        role: None,
        role_span: None,
        role_resolution: ArgumentRoleResolution::Unspecified,
        expression: receiver.clone(),
    });
    combined.extend(arguments.iter().cloned());
    combined
}

fn lower_fat_pointer_method(
    function: &mut FunctionBuilder<'_>,
    receiver: &Expr,
    context: &CallLoweringContext<'_, '_>,
) -> Result<Value, NativeEmitError> {
    let fat_pointer = lower_expression(
        function,
        receiver,
        context.locals,
        context.local_types,
        context.functions,
        context.cleanup_schedule,
        context.string_data,
        context.layouts,
    )?;
    lower_dynamic_call(function, fat_pointer)
}

fn method_callee(
    receiver_type: NativeType,
    method: &str,
    functions: &std::collections::HashMap<String, super::super::native::FunctionRef>,
) -> String {
    let dispatch_name = dispatch_key(receiver_type, method);
    if functions.contains_key(&dispatch_name) { dispatch_name } else { method.to_owned() }
}

fn receiver_native_type(
    receiver: &Expr,
    context: &CallLoweringContext<'_, '_>,
) -> Result<NativeType, NativeEmitError> {
    super::super::expressions::initializer_type(
        receiver,
        context.local_types,
        context.functions,
        context.layouts,
    )
}
