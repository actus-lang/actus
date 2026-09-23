use std::collections::HashMap;

use cranelift_codegen::ir::InstBuilder;
use cranelift_frontend::FunctionBuilder;

use crate::ast::{BinaryOp, Expr, UnaryOp};

use super::expressions::lower_expression;
use super::layout::LayoutRegistry;
use super::literals::StringDataValues;
use super::model::NativeCleanupSchedule;
use super::native::{FunctionRef, NativeEmitError};
use super::types::NativeType;

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_operation(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    match expression {
        Expr::Unary { operator, expression, .. } => lower_unary(
            function,
            operator,
            expression,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
        Expr::Binary { left, operator, right, .. } => lower_binary(
            function,
            left,
            operator,
            right,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
        _ => Err(NativeEmitError("unsupported native operation".to_owned())),
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_unary(
    function: &mut FunctionBuilder<'_>,
    operator: &UnaryOp,
    expression: &Expr,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let value = lower_expression(
        function,
        expression,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    match operator {
        UnaryOp::Negate => Ok(function.ins().ineg(value)),
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_binary(
    function: &mut FunctionBuilder<'_>,
    left: &Expr,
    operator: &BinaryOp,
    right: &Expr,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let left = lower_expression(
        function,
        left,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    let right = lower_expression(
        function,
        right,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    Ok(match operator {
        BinaryOp::Add => function.ins().iadd(left, right),
        BinaryOp::Subtract => function.ins().isub(left, right),
        BinaryOp::Multiply => function.ins().imul(left, right),
        BinaryOp::Divide => function.ins().sdiv(left, right),
    })
}
