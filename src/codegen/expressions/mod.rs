use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, Value};
use cranelift_frontend::FunctionBuilder;

use crate::ast::Expr;

use super::case::lower_case;
use super::layout::LayoutRegistry;
use super::literals::StringDataValues;
use super::model::NativeCleanupSchedule;
use super::native::{FunctionRef, NativeEmitError};
use super::types::NativeType;

mod buffer;
mod construct;
mod initializer_types;
mod literals;
mod operations;
mod try_lowering;

pub(super) use buffer::emit_buffer_drop;
pub(super) use construct::lower_construct;
pub(super) use initializer_types::initializer_type;
pub(super) use literals::{
    coerce_to_ir_type, lower_float, lower_float_as, lower_identifier, lower_integer, lower_string,
    lower_wide_integer,
};
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
    match expression {
        Expr::Integer { value, .. } => lower_integer(function, value),
        Expr::BufferLiteral { length, .. } => lower_buffer_literal(
            function,
            length,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
        Expr::FloatLiteral { value, .. } => lower_float(function, value),
        Expr::StringLiteral { value, .. } => lower_string(function, value, string_data),
        Expr::Identifier { name, .. } => lower_identifier(name, locals),
        Expr::Grouping { expression, .. } | Expr::Borrow { expression, .. } => lower_expression(
            function,
            expression,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
        _ => lower_complex_expression(
            function,
            expression,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_buffer_literal(
    function: &mut FunctionBuilder<'_>,
    length: &Expr,
    locals: &HashMap<&String, Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Value, NativeEmitError> {
    let length = lower_expression(
        function,
        length,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    let length = if function.func.dfg.value_type(length) == layouts.pointer_type {
        length
    } else {
        function.ins().uextend(layouts.pointer_type, length)
    };
    let allocator = functions
        .get("__actus_buffer_allocate")
        .ok_or_else(|| NativeEmitError("native buffer allocator is unavailable".to_owned()))?;
    let call = function.ins().call(allocator.reference, &[length]);
    function
        .inst_results(call)
        .first()
        .copied()
        .ok_or_else(|| NativeEmitError("native buffer allocator returned no value".to_owned()))
}

#[allow(clippy::too_many_arguments)]
fn lower_complex_expression(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    locals: &HashMap<&String, Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Value, NativeEmitError> {
    match expression {
        Expr::Try { expression, span } => lower_try_expression(
            function,
            expression,
            *span,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
        Expr::Unary { .. } | Expr::Binary { .. } => lower_operation(
            function,
            expression,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
        _ => lower_construct_or_case(
            function,
            expression,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_construct_or_case(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    locals: &HashMap<&String, Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Value, NativeEmitError> {
    match expression {
        Expr::Call { .. }
        | Expr::MethodCall { .. }
        | Expr::StructLit { .. }
        | Expr::FieldAccess { .. } => lower_construct(
            function,
            expression,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
        Expr::Case { subject, branches, .. } => lower_case(
            function,
            subject,
            branches,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
        _ => Err(NativeEmitError("unsupported native expression".to_owned())),
    }
}
