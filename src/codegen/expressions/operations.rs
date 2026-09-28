use cranelift_codegen::ir::InstBuilder;
use cranelift_frontend::FunctionBuilder;

use crate::ast::{BinaryOp, Expr, UnaryOp};

use super::super::calls::CallLoweringContext;
use super::super::native::NativeEmitError;
use super::lower_expression;

pub(in crate::codegen) fn lower_operation(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    context: &CallLoweringContext<'_, '_>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    match expression {
        Expr::Unary { operator, expression, .. } => {
            lower_unary(function, operator, expression, context)
        }
        Expr::Binary { left, operator, right, .. } => {
            lower_binary(function, left, operator, right, context)
        }
        _ => Err(NativeEmitError("unsupported native operation".to_owned())),
    }
}

fn lower_unary(
    function: &mut FunctionBuilder<'_>,
    operator: &UnaryOp,
    expression: &Expr,
    context: &CallLoweringContext<'_, '_>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let value = lower_expression_with_context(function, expression, context)?;
    match operator {
        UnaryOp::Negate => Ok(function.ins().ineg(value)),
    }
}

fn lower_binary(
    function: &mut FunctionBuilder<'_>,
    left: &Expr,
    operator: &BinaryOp,
    right: &Expr,
    context: &CallLoweringContext<'_, '_>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let left = lower_expression_with_context(function, left, context)?;
    let right = lower_expression_with_context(function, right, context)?;
    Ok(match operator {
        BinaryOp::Add => function.ins().iadd(left, right),
        BinaryOp::Subtract => function.ins().isub(left, right),
        BinaryOp::Multiply => function.ins().imul(left, right),
        BinaryOp::Divide => function.ins().sdiv(left, right),
    })
}

fn lower_expression_with_context(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    context: &CallLoweringContext<'_, '_>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    lower_expression(
        function,
        expression,
        context.locals,
        context.local_types,
        context.functions,
        context.cleanup_schedule,
        context.string_data,
        context.layouts,
    )
}
