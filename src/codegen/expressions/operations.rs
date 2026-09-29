use cranelift_codegen::ir::{
    InstBuilder,
    condcodes::{FloatCC, IntCC},
};
use cranelift_frontend::FunctionBuilder;

use crate::ast::{BinaryOp, Expr, UnaryOp};

use super::super::calls::CallLoweringContext;
use super::super::native::NativeEmitError;
use super::super::types::NativeType;
use super::initializer_type;
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
        UnaryOp::LogicalNot | UnaryOp::BitwiseNot => {
            Err(NativeEmitError("operator is not available in native lowering yet".to_owned()))
        }
    }
}

fn lower_binary(
    function: &mut FunctionBuilder<'_>,
    left: &Expr,
    operator: &BinaryOp,
    right: &Expr,
    context: &CallLoweringContext<'_, '_>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    if operator.is_relational() {
        return lower_relational(function, left, operator, right, context);
    }
    let left = lower_expression_with_context(function, left, context)?;
    let right = lower_expression_with_context(function, right, context)?;
    Ok(match operator {
        BinaryOp::Add => function.ins().iadd(left, right),
        BinaryOp::Subtract => function.ins().isub(left, right),
        BinaryOp::Multiply => function.ins().imul(left, right),
        BinaryOp::Divide => function.ins().sdiv(left, right),
        _ => unreachable!("relational operators are lowered above"),
    })
}

fn lower_relational(
    function: &mut FunctionBuilder<'_>,
    left_expression: &Expr,
    operator: &BinaryOp,
    right_expression: &Expr,
    context: &CallLoweringContext<'_, '_>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let left = lower_expression_with_context(function, left_expression, context)?;
    let right = lower_expression_with_context(function, right_expression, context)?;
    let left_type =
        initializer_type(left_expression, context.local_types, context.functions, context.layouts)?;
    let condition = match left_type {
        NativeType::Float { .. } => function.ins().fcmp(float_condition(operator)?, left, right),
        NativeType::Int | NativeType::Integer { .. } => {
            let right_type = initializer_type(
                right_expression,
                context.local_types,
                context.functions,
                context.layouts,
            )?;
            let (left, right) = widen_integer_operands(
                function,
                left,
                right,
                left_type,
                right_type,
                context.layouts.pointer_type,
            )?;
            function.ins().icmp(integer_condition(operator, left_type)?, left, right)
        }
        other => return Err(NativeEmitError(format!("cannot compare native type {other:?}"))),
    };
    Ok(condition)
}

fn widen_integer_operands(
    function: &mut FunctionBuilder<'_>,
    left: cranelift_codegen::ir::Value,
    right: cranelift_codegen::ir::Value,
    left_type: NativeType,
    right_type: NativeType,
    pointer_type: cranelift_codegen::ir::Type,
) -> Result<(cranelift_codegen::ir::Value, cranelift_codegen::ir::Value), NativeEmitError> {
    let left_ir = left_type.ir_type(pointer_type)?;
    let right_ir = right_type.ir_type(pointer_type)?;
    let target = if left_ir.bytes() >= right_ir.bytes() { left_ir } else { right_ir };
    Ok((
        extend_integer(function, left, left_ir, target, is_signed(left_type)),
        extend_integer(function, right, right_ir, target, is_signed(right_type)),
    ))
}

fn extend_integer(
    function: &mut FunctionBuilder<'_>,
    value: cranelift_codegen::ir::Value,
    source: cranelift_codegen::ir::Type,
    target: cranelift_codegen::ir::Type,
    signed: bool,
) -> cranelift_codegen::ir::Value {
    if source == target {
        value
    } else if signed {
        function.ins().sextend(target, value)
    } else {
        function.ins().uextend(target, value)
    }
}

fn is_signed(ty: NativeType) -> bool {
    matches!(ty, NativeType::Int | NativeType::Integer { signed: true, .. })
}

fn integer_condition(operator: &BinaryOp, ty: NativeType) -> Result<IntCC, NativeEmitError> {
    let signed = matches!(ty, NativeType::Int | NativeType::Integer { signed: true, .. });
    Ok(match (operator, signed) {
        (BinaryOp::LessThan, true) => IntCC::SignedLessThan,
        (BinaryOp::LessEquals, true) => IntCC::SignedLessThanOrEqual,
        (BinaryOp::GreaterThan, true) => IntCC::SignedGreaterThan,
        (BinaryOp::GreaterEquals, true) => IntCC::SignedGreaterThanOrEqual,
        (BinaryOp::LessThan, false) => IntCC::UnsignedLessThan,
        (BinaryOp::LessEquals, false) => IntCC::UnsignedLessThanOrEqual,
        (BinaryOp::GreaterThan, false) => IntCC::UnsignedGreaterThan,
        (BinaryOp::GreaterEquals, false) => IntCC::UnsignedGreaterThanOrEqual,
        _ => {
            return Err(NativeEmitError(
                "non-relational operator in comparison lowering".to_owned(),
            ));
        }
    })
}

fn float_condition(operator: &BinaryOp) -> Result<FloatCC, NativeEmitError> {
    Ok(match operator {
        BinaryOp::LessThan => FloatCC::LessThan,
        BinaryOp::LessEquals => FloatCC::LessThanOrEqual,
        BinaryOp::GreaterThan => FloatCC::GreaterThan,
        BinaryOp::GreaterEquals => FloatCC::GreaterThanOrEqual,
        _ => {
            return Err(NativeEmitError(
                "non-relational operator in comparison lowering".to_owned(),
            ));
        }
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
