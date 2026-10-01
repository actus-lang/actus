use cranelift_codegen::ir::{
    InstBuilder, TrapCode,
    condcodes::{FloatCC, IntCC},
    types,
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
        UnaryOp::LogicalNot => {
            let value_type = function.func.dfg.value_type(value);
            let zero = function.ins().iconst(value_type, 0);
            let inverted = function.ins().icmp(IntCC::Equal, value, zero);
            Ok(normalize_bool(function, inverted))
        }
        UnaryOp::BitwiseNot => Ok(function.ins().bnot(value)),
    }
}

fn lower_binary(
    function: &mut FunctionBuilder<'_>,
    left: &Expr,
    operator: &BinaryOp,
    right: &Expr,
    context: &CallLoweringContext<'_, '_>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    if operator.is_relational() || matches!(operator, BinaryOp::Equals | BinaryOp::NotEquals) {
        return lower_comparison(function, left, operator, right, context);
    }
    if matches!(operator, BinaryOp::LogicalAnd | BinaryOp::LogicalOr) {
        return super::short_circuit::lower_short_circuit(function, left, operator, right, context);
    }
    lower_integer_operation(function, left, operator, right, context)
}

fn lower_comparison(
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
    Ok(normalize_bool(function, condition))
}

fn normalize_bool(
    function: &mut FunctionBuilder<'_>,
    value: cranelift_codegen::ir::Value,
) -> cranelift_codegen::ir::Value {
    if function.func.dfg.value_type(value) == types::I32 {
        value
    } else {
        function.ins().uextend(types::I32, value)
    }
}

fn lower_integer_operation(
    function: &mut FunctionBuilder<'_>,
    left_expression: &Expr,
    operator: &BinaryOp,
    right_expression: &Expr,
    context: &CallLoweringContext<'_, '_>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let left_type =
        initializer_type(left_expression, context.local_types, context.functions, context.layouts)?;
    let right_type = initializer_type(
        right_expression,
        context.local_types,
        context.functions,
        context.layouts,
    )?;
    let left = lower_expression_with_context(function, left_expression, context)?;
    let right = lower_expression_with_context(function, right_expression, context)?;
    let (left, right) = widen_integer_operands(
        function,
        left,
        right,
        left_type,
        right_type,
        context.layouts.pointer_type,
    )?;
    if matches!(operator, BinaryOp::Divide | BinaryOp::Remainder) {
        trap_if_zero(function, right);
    }
    if matches!(operator, BinaryOp::ShiftLeft | BinaryOp::ShiftRight) {
        trap_if_shift_count_exceeds_width(function, right, left);
    }
    Ok(match operator {
        BinaryOp::Add => function.ins().iadd(left, right),
        BinaryOp::Subtract => function.ins().isub(left, right),
        BinaryOp::Multiply => function.ins().imul(left, right),
        BinaryOp::Divide => integer_divide(function, left, right, left_type),
        BinaryOp::Remainder => integer_remainder(function, left, right, left_type),
        BinaryOp::BitwiseAnd => function.ins().band(left, right),
        BinaryOp::BitwiseOr => function.ins().bor(left, right),
        BinaryOp::BitwiseXor => function.ins().bxor(left, right),
        BinaryOp::ShiftLeft => function.ins().ishl(left, right),
        BinaryOp::ShiftRight if is_signed(left_type) => function.ins().sshr(left, right),
        BinaryOp::ShiftRight => function.ins().ushr(left, right),
        _ => return Err(NativeEmitError("unsupported integer operator".to_owned())),
    })
}

pub(in crate::codegen) fn lower_compound_integer_operation(
    function: &mut FunctionBuilder<'_>,
    left: cranelift_codegen::ir::Value,
    operator: BinaryOp,
    right: cranelift_codegen::ir::Value,
    left_type: NativeType,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    if matches!(operator, BinaryOp::Divide | BinaryOp::Remainder) {
        trap_if_zero(function, right);
    }
    if matches!(operator, BinaryOp::ShiftLeft | BinaryOp::ShiftRight) {
        trap_if_shift_count_exceeds_width(function, right, left);
    }
    Ok(match operator {
        BinaryOp::Add => function.ins().iadd(left, right),
        BinaryOp::Subtract => function.ins().isub(left, right),
        BinaryOp::Multiply => function.ins().imul(left, right),
        BinaryOp::Divide => integer_divide(function, left, right, left_type),
        BinaryOp::Remainder => integer_remainder(function, left, right, left_type),
        BinaryOp::BitwiseAnd => function.ins().band(left, right),
        BinaryOp::BitwiseOr => function.ins().bor(left, right),
        BinaryOp::BitwiseXor => function.ins().bxor(left, right),
        BinaryOp::ShiftLeft => function.ins().ishl(left, right),
        BinaryOp::ShiftRight if is_signed(left_type) => function.ins().sshr(left, right),
        BinaryOp::ShiftRight => function.ins().ushr(left, right),
        _ => return Err(NativeEmitError("unsupported compound operator".to_owned())),
    })
}

fn integer_divide(
    function: &mut FunctionBuilder<'_>,
    left: cranelift_codegen::ir::Value,
    right: cranelift_codegen::ir::Value,
    left_type: NativeType,
) -> cranelift_codegen::ir::Value {
    if is_signed(left_type) {
        function.ins().sdiv(left, right)
    } else {
        function.ins().udiv(left, right)
    }
}

fn integer_remainder(
    function: &mut FunctionBuilder<'_>,
    left: cranelift_codegen::ir::Value,
    right: cranelift_codegen::ir::Value,
    left_type: NativeType,
) -> cranelift_codegen::ir::Value {
    if is_signed(left_type) {
        function.ins().srem(left, right)
    } else {
        function.ins().urem(left, right)
    }
}

fn trap_if_zero(function: &mut FunctionBuilder<'_>, value: cranelift_codegen::ir::Value) {
    let value_type = function.func.dfg.value_type(value);
    let zero = function.ins().iconst(value_type, 0);
    let nonzero = function.ins().icmp(IntCC::NotEqual, value, zero);
    let ok_block = function.create_block();
    let trap_block = function.create_block();
    function.ins().brif(nonzero, ok_block, &[], trap_block, &[]);
    function.switch_to_block(trap_block);
    function.ins().trap(TrapCode::INTEGER_DIVISION_BY_ZERO);
    function.seal_block(trap_block);
    function.switch_to_block(ok_block);
    function.seal_block(ok_block);
}

fn trap_if_shift_count_exceeds_width(
    function: &mut FunctionBuilder<'_>,
    count: cranelift_codegen::ir::Value,
    operand: cranelift_codegen::ir::Value,
) {
    let value_type = function.func.dfg.value_type(count);
    let width = i64::from(function.func.dfg.value_type(operand).bits());
    let limit = function.ins().iconst(value_type, width);
    let valid = function.ins().icmp(IntCC::UnsignedLessThan, count, limit);
    let ok_block = function.create_block();
    let trap_block = function.create_block();
    function.ins().brif(valid, ok_block, &[], trap_block, &[]);
    function.switch_to_block(trap_block);
    function.ins().trap(TrapCode::INTEGER_OVERFLOW);
    function.seal_block(trap_block);
    function.switch_to_block(ok_block);
    function.seal_block(ok_block);
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
        (BinaryOp::Equals, _) => IntCC::Equal,
        (BinaryOp::NotEquals, _) => IntCC::NotEqual,
        (BinaryOp::LessThan, true) => IntCC::SignedLessThan,
        (BinaryOp::LessEquals, true) => IntCC::SignedLessThanOrEqual,
        (BinaryOp::GreaterThan, true) => IntCC::SignedGreaterThan,
        (BinaryOp::GreaterEquals, true) => IntCC::SignedGreaterThanOrEqual,
        (BinaryOp::LessThan, false) => IntCC::UnsignedLessThan,
        (BinaryOp::LessEquals, false) => IntCC::UnsignedLessThanOrEqual,
        (BinaryOp::GreaterThan, false) => IntCC::UnsignedGreaterThan,
        (BinaryOp::GreaterEquals, false) => IntCC::UnsignedGreaterThanOrEqual,
        _ => {
            return Err(NativeEmitError("unsupported integer comparison operator".to_owned()));
        }
    })
}

fn float_condition(operator: &BinaryOp) -> Result<FloatCC, NativeEmitError> {
    Ok(match operator {
        BinaryOp::Equals => FloatCC::Equal,
        BinaryOp::NotEquals => FloatCC::NotEqual,
        BinaryOp::LessThan => FloatCC::LessThan,
        BinaryOp::LessEquals => FloatCC::LessThanOrEqual,
        BinaryOp::GreaterThan => FloatCC::GreaterThan,
        BinaryOp::GreaterEquals => FloatCC::GreaterThanOrEqual,
        _ => {
            return Err(NativeEmitError("unsupported float comparison operator".to_owned()));
        }
    })
}

pub(super) fn lower_expression_with_context(
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
