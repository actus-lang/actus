use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, Value};
use cranelift_frontend::FunctionBuilder;

use crate::ast::Expr;

use super::calls::CallLoweringContext;
use super::case::lower_case;
use super::layout::LayoutRegistry;
use super::literals::StringDataValues;
use super::model::NativeCleanupSchedule;
use super::native::{FunctionRef, NativeEmitError};
use super::types::NativeType;

mod buffer;
mod cast;
mod construct;
mod if_lowering;
mod initializer_types;
mod literals;
mod operations;
mod short_circuit;
mod try_lowering;

pub(super) use buffer::emit_buffer_drop;
pub(super) use construct::lower_construct;
pub(super) use initializer_types::initializer_type;
pub(super) use literals::{
    coerce_to_ir_type, lower_float, lower_float_as, lower_identifier, lower_integer, lower_string,
    lower_wide_integer,
};
pub(super) use operations::lower_compound_integer_operation;
pub(super) use operations::lower_operation;
pub(super) use try_lowering::lower_try_expression;

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_expression(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    locals: &HashMap<&String, Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Value, NativeEmitError> {
    lower_expression_with_targets(
        function,
        expression,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_expression_with_targets(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    locals: &HashMap<&String, Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
    loop_targets: Option<super::lowering::LoopTargets>,
) -> Result<Value, NativeEmitError> {
    let context = CallLoweringContext::new(
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )
    .with_loop_targets(loop_targets);
    lower_expression_with_context(function, expression, &context)
}

fn lower_expression_with_context(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    context: &CallLoweringContext<'_, '_>,
) -> Result<Value, NativeEmitError> {
    match expression {
        Expr::BoolLiteral { value, .. } => lower_bool_literal(function, *value),
        Expr::Integer { value, suffix, .. } => {
            lower_integer_expression(function, value, suffix.as_deref(), context)
        }
        Expr::BufferLiteral { length, .. } => lower_buffer_literal(function, length, context),
        Expr::FloatLiteral { value, suffix, .. } => match suffix.as_deref() {
            Some("f32") => lower_float_as(function, value, cranelift_codegen::ir::types::F32),
            _ => lower_float(function, value),
        },
        Expr::StringLiteral { value, .. } => lower_string(function, value, context.string_data),
        Expr::Identifier { name, .. } => lower_identifier(
            function,
            name,
            context.locals,
            context.local_types,
            context.layouts.pointer_type,
        ),
        Expr::Grouping { expression, .. } | Expr::Borrow { expression, .. } => {
            lower_expression_with_context(function, expression, context)
        }
        Expr::Cast { expression, target, .. } => {
            cast::lower_cast(function, expression, target, context)
        }
        Expr::Index { target, index, .. } => {
            lower_index_expression(function, target, index, context)
        }
        _ => lower_complex_expression(function, expression, context),
    }
}

fn lower_bool_literal(
    function: &mut FunctionBuilder<'_>,
    value: bool,
) -> Result<Value, NativeEmitError> {
    Ok(function.ins().iconst(cranelift_codegen::ir::types::I32, i64::from(value)))
}

fn lower_index_expression(
    function: &mut FunctionBuilder<'_>,
    target: &Expr,
    index: &Expr,
    context: &CallLoweringContext<'_, '_>,
) -> Result<Value, NativeEmitError> {
    let lower = match super::structs::expression_native_type(
        target,
        context.local_types,
        context.layouts,
    ) {
        Some(NativeType::Buffer) => super::buffer_index::lower_buffer_index,
        _ => super::arrays::lower_array_index,
    };
    lower(
        function,
        target,
        index,
        context.locals,
        context.local_types,
        context.functions,
        context.cleanup_schedule,
        context.string_data,
        context.layouts,
    )
}

fn lower_integer_expression(
    function: &mut FunctionBuilder<'_>,
    value: &str,
    suffix: Option<&str>,
    context: &CallLoweringContext<'_, '_>,
) -> Result<Value, NativeEmitError> {
    let value = lower_integer(function, value)?;
    let Some(suffix) = suffix else { return Ok(value) };
    let Some(native_type) = NativeType::from_name(suffix) else { return Ok(value) };
    Ok(coerce_to_ir_type(function, value, native_type.ir_type(context.layouts.pointer_type)?))
}

fn lower_buffer_literal(
    function: &mut FunctionBuilder<'_>,
    length: &Expr,
    context: &CallLoweringContext<'_, '_>,
) -> Result<Value, NativeEmitError> {
    let length = lower_expression_with_context(function, length, context)?;
    let length = if function.func.dfg.value_type(length) == context.layouts.pointer_type {
        length
    } else {
        function.ins().uextend(context.layouts.pointer_type, length)
    };
    let allocator = context
        .functions
        .get("__actus_buffer_allocate")
        .ok_or_else(|| NativeEmitError("native buffer allocator is unavailable".to_owned()))?;
    let call = function.ins().call(allocator.reference, &[length]);
    function
        .inst_results(call)
        .first()
        .copied()
        .ok_or_else(|| NativeEmitError("native buffer allocator returned no value".to_owned()))
}

fn lower_complex_expression(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    context: &CallLoweringContext<'_, '_>,
) -> Result<Value, NativeEmitError> {
    match expression {
        Expr::Try { expression, span } => {
            lower_try_expression(function, expression, *span, context)
        }
        Expr::If { .. } => if_lowering::lower_if(function, expression, context),
        Expr::Unary { .. } | Expr::Binary { .. } => lower_operation(function, expression, context),
        _ => lower_construct_or_case(function, expression, context),
    }
}

fn lower_construct_or_case(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    context: &CallLoweringContext<'_, '_>,
) -> Result<Value, NativeEmitError> {
    match expression {
        Expr::Call { .. }
        | Expr::MethodCall { .. }
        | Expr::StructLit { .. }
        | Expr::FieldAccess { .. } => lower_construct(function, expression, context),
        Expr::Case { subject, branches, .. } => {
            lower_case_expression(function, subject, branches, context)
        }
        _ => Err(NativeEmitError("unsupported native expression".to_owned())),
    }
}

fn lower_case_expression(
    function: &mut FunctionBuilder<'_>,
    subject: &Expr,
    branches: &[crate::ast::CaseBranch],
    context: &CallLoweringContext<'_, '_>,
) -> Result<Value, NativeEmitError> {
    lower_case(
        function,
        subject,
        branches,
        context.locals,
        context.local_types,
        context.functions,
        context.cleanup_schedule,
        context.string_data,
        context.layouts,
        context.loop_targets.clone(),
    )
}
