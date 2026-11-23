use std::collections::BTreeMap;

use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{InstBuilder, TrapCode, types};
use cranelift_frontend::FunctionBuilder;
use cranelift_object::ObjectModule;

use super::super::checks::{check_failed_if, emit_check_failure};

use crate::codegen::{
    BackendEmitError, BackendInstruction, BackendIntegerBinaryOp, BackendIntegerCompareOp,
    BackendIntegerUnaryOp, BackendValueId, BackendValueType,
};

pub(in crate::codegen::cranelift) fn lower_integer_binary(
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
    module: &mut ObjectModule,
    failure: Option<&str>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::IntegerBinary { result, operation, left, right, ty, .. } = instruction
    else {
        return Err(BackendEmitError("Cranelift integer operation expected".to_owned()));
    };
    let left = values.get(left).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift binary left operand {} is unavailable", left.0))
    })?;
    let right = values.get(right).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift binary right operand {} is unavailable", right.0))
    })?;
    let right = coerce_integer_width(builder, right, builder.func.dfg.value_type(left));
    let value = match arithmetic(builder, *operation, *ty, (left, right), (module, failure))? {
        Some(value) => value,
        None => match operation {
            BackendIntegerBinaryOp::ShiftLeft => builder.ins().ishl(left, right),
            BackendIntegerBinaryOp::ShiftRight if is_signed(*ty) => builder.ins().sshr(left, right),
            BackendIntegerBinaryOp::ShiftRight => builder.ins().ushr(left, right),
            BackendIntegerBinaryOp::BitwiseAnd | BackendIntegerBinaryOp::LogicalAnd => {
                builder.ins().band(left, right)
            }
            BackendIntegerBinaryOp::BitwiseOr | BackendIntegerBinaryOp::LogicalOr => {
                builder.ins().bor(left, right)
            }
            _ => builder.ins().bxor(left, right),
        },
    };
    let value = if matches!(ty, BackendValueType::Bool) {
        let value_type = builder.func.dfg.value_type(value);
        let zero = builder.ins().iconst(value_type, 0);
        builder.ins().icmp(IntCC::NotEqual, value, zero)
    } else {
        value
    };
    values.insert(*result, value);
    Ok(())
}

/// The value of an arithmetic operation, or `None` for a shift, bitwise or
/// logical operation.
fn arithmetic(
    builder: &mut FunctionBuilder<'_>,
    operation: BackendIntegerBinaryOp,
    ty: BackendValueType,
    (left, right): (cranelift_codegen::ir::Value, cranelift_codegen::ir::Value),
    (module, failure): (&mut ObjectModule, Option<&str>),
) -> Result<Option<cranelift_codegen::ir::Value>, BackendEmitError> {
    let signed = is_signed(ty);
    Ok(Some(match operation {
        BackendIntegerBinaryOp::Add => builder.ins().iadd(left, right),
        BackendIntegerBinaryOp::Subtract => builder.ins().isub(left, right),
        BackendIntegerBinaryOp::Multiply => builder.ins().imul(left, right),
        BackendIntegerBinaryOp::TrappingAdd
        | BackendIntegerBinaryOp::TrappingSubtract
        | BackendIntegerBinaryOp::TrappingMultiply => {
            let (value, overflow) = overflowing(builder, operation, signed, left, right);
            check_failed_if(module, builder, overflow, TrapCode::INTEGER_OVERFLOW, failure)?;
            value
        }
        BackendIntegerBinaryOp::SaturatingAdd
        | BackendIntegerBinaryOp::SaturatingSubtract
        | BackendIntegerBinaryOp::SaturatingMultiply => {
            saturating(builder, operation, signed, left, right)
        }
        BackendIntegerBinaryOp::Divide if signed => builder.ins().sdiv(left, right),
        BackendIntegerBinaryOp::Divide => builder.ins().udiv(left, right),
        BackendIntegerBinaryOp::Remainder if signed => builder.ins().srem(left, right),
        BackendIntegerBinaryOp::Remainder => builder.ins().urem(left, right),
        _ => return Ok(None),
    }))
}

/// The wrapped result of an add, subtract or multiply and whether the exact
/// result overflowed the operand type (ADR-0115).
pub(in crate::codegen::cranelift) fn overflowing(
    builder: &mut FunctionBuilder<'_>,
    operation: BackendIntegerBinaryOp,
    signed: bool,
    left: cranelift_codegen::ir::Value,
    right: cranelift_codegen::ir::Value,
) -> (cranelift_codegen::ir::Value, cranelift_codegen::ir::Value) {
    use BackendIntegerBinaryOp::{
        SaturatingAdd, SaturatingSubtract, TrappingAdd, TrappingSubtract,
    };
    let instructions = builder.ins();
    match (operation, signed) {
        (TrappingAdd | SaturatingAdd, true) => instructions.sadd_overflow(left, right),
        (TrappingAdd | SaturatingAdd, false) => instructions.uadd_overflow(left, right),
        (TrappingSubtract | SaturatingSubtract, true) => instructions.ssub_overflow(left, right),
        (TrappingSubtract | SaturatingSubtract, false) => instructions.usub_overflow(left, right),
        (_, true) => instructions.smul_overflow(left, right),
        (_, false) => instructions.umul_overflow(left, right),
    }
}

/// The result clamped to the bound the exact result crossed: the maximum,
/// or for a signed operation whose exact result is negative, the minimum;
/// an unsigned subtraction clamps at zero.
pub(in crate::codegen::cranelift) fn saturating(
    builder: &mut FunctionBuilder<'_>,
    operation: BackendIntegerBinaryOp,
    signed: bool,
    left: cranelift_codegen::ir::Value,
    right: cranelift_codegen::ir::Value,
) -> cranelift_codegen::ir::Value {
    let (value, overflow) = overflowing(builder, operation, signed, left, right);
    let ty = builder.func.dfg.value_type(value);
    let bits = ty.bits();
    let maximum = if signed { (1i128 << (bits - 1)) - 1 } else { (1i128 << bits) - 1 };
    let maximum = builder.ins().iconst(ty, maximum as i64);
    let bound = if !signed {
        if operation == BackendIntegerBinaryOp::SaturatingSubtract {
            builder.ins().iconst(ty, 0)
        } else {
            maximum
        }
    } else {
        let minimum = builder.ins().iconst(ty, (-(1i128 << (bits - 1))) as i64);
        let sign_source = match operation {
            BackendIntegerBinaryOp::SaturatingMultiply => builder.ins().bxor(left, right),
            _ => right,
        };
        let zero = builder.ins().iconst(ty, 0);
        let negative = builder.ins().icmp(IntCC::SignedLessThan, sign_source, zero);
        match operation {
            BackendIntegerBinaryOp::SaturatingSubtract => {
                builder.ins().select(negative, maximum, minimum)
            }
            _ => builder.ins().select(negative, minimum, maximum),
        }
    };
    builder.ins().select(overflow, bound, value)
}

pub(in crate::codegen::cranelift) fn lower_integer_cast(
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
    module: &mut ObjectModule,
    failure: Option<&str>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::IntegerCast { result, source, source_type, target_type, .. } =
        instruction
    else {
        return Err(BackendEmitError("Cranelift integer cast expected".to_owned()));
    };
    let source_value = values.get(source).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift cast source {} is unavailable", source.0))
    })?;
    let wide = widen_integer(builder, source_value, *source_type)?;
    let valid = cast_validity(builder, wide, *source_type, *target_type)?;
    let ok_block = builder.create_block();
    let trap_block = builder.create_block();
    builder.ins().brif(valid, ok_block, &[], trap_block, &[]);
    builder.switch_to_block(trap_block);
    emit_check_failure(module, builder, TrapCode::INTEGER_OVERFLOW, failure)?;
    builder.seal_block(trap_block);
    builder.switch_to_block(ok_block);
    builder.seal_block(ok_block);
    let target_ir = integer_ir_type(*target_type)?;
    let converted =
        if target_ir == types::I64 { wide } else { builder.ins().ireduce(target_ir, wide) };
    values.insert(*result, converted);
    Ok(())
}

fn widen_integer(
    builder: &mut FunctionBuilder<'_>,
    value: cranelift_codegen::ir::Value,
    ty: BackendValueType,
) -> Result<cranelift_codegen::ir::Value, BackendEmitError> {
    let signed = match ty {
        BackendValueType::Integer { signed, .. } => signed,
        BackendValueType::Bool => false,
        _ => return Err(BackendEmitError("integer cast source must be an integer".to_owned())),
    };
    Ok(if builder.func.dfg.value_type(value) == types::I64 {
        value
    } else if signed {
        builder.ins().sextend(types::I64, value)
    } else {
        builder.ins().uextend(types::I64, value)
    })
}

fn cast_validity(
    builder: &mut FunctionBuilder<'_>,
    value: cranelift_codegen::ir::Value,
    source: BackendValueType,
    target: BackendValueType,
) -> Result<cranelift_codegen::ir::Value, BackendEmitError> {
    let source_signed = match source {
        BackendValueType::Bool => false,
        BackendValueType::Integer { signed, .. } => signed,
        _ => return Err(BackendEmitError("integer cast types are invalid".to_owned())),
    };
    let BackendValueType::Integer { signed: target_signed, width: target_width } = target else {
        return Err(BackendEmitError("integer cast types are invalid".to_owned()));
    };
    let zero = builder.ins().iconst(types::I64, 0);
    let lower = if source_signed && !target_signed {
        builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, value, zero)
    } else {
        builder.ins().iconst(types::I8, 1)
    };
    let maximum = if target_signed {
        if target_width == 64 { i64::MAX } else { (1i64 << (target_width - 1)) - 1 }
    } else if target_width == 64 {
        i64::MAX
    } else {
        (1i64 << target_width) - 1
    };
    let limit = builder.ins().iconst(types::I64, maximum);
    let upper = if !target_signed && target_width == 64 && !source_signed {
        builder.ins().iconst(types::I8, 1)
    } else {
        let predicate = if source_signed {
            IntCC::SignedLessThanOrEqual
        } else {
            IntCC::UnsignedLessThanOrEqual
        };
        builder.ins().icmp(predicate, value, limit)
    };
    Ok(builder.ins().band(lower, upper))
}

fn integer_ir_type(ty: BackendValueType) -> Result<types::Type, BackendEmitError> {
    match ty {
        BackendValueType::Integer { width: 8, .. } => Ok(types::I8),
        BackendValueType::Integer { width: 16, .. } => Ok(types::I16),
        BackendValueType::Integer { width: 32, .. } => Ok(types::I32),
        BackendValueType::Integer { width: 64, .. } => Ok(types::I64),
        _ => Err(BackendEmitError("integer cast target must be an integer".to_owned())),
    }
}

pub(in crate::codegen::cranelift) fn lower_trap_if_zero(
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
    module: &mut ObjectModule,
    failure: Option<&str>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::TrapIfZero { value, .. } = instruction else {
        return Err(BackendEmitError("Cranelift zero guard expected".to_owned()));
    };
    let value = values.get(value).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift zero-guard value {} is unavailable", value.0))
    })?;
    let value_type = builder.func.dfg.value_type(value);
    let zero = builder.ins().iconst(value_type, 0);
    let nonzero = builder.ins().icmp(IntCC::NotEqual, value, zero);
    let ok_block = builder.create_block();
    let trap_block = builder.create_block();
    builder.ins().brif(nonzero, ok_block, &[], trap_block, &[]);
    builder.switch_to_block(trap_block);
    emit_check_failure(module, builder, TrapCode::INTEGER_DIVISION_BY_ZERO, failure)?;
    builder.seal_block(trap_block);
    builder.switch_to_block(ok_block);
    builder.seal_block(ok_block);
    Ok(())
}

pub(in crate::codegen::cranelift) fn lower_trap_if_shift_count_out_of_range(
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
    module: &mut ObjectModule,
    failure: Option<&str>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::TrapIfShiftCountOutOfRange { count, width, .. } = instruction else {
        return Err(BackendEmitError("Cranelift shift-count guard expected".to_owned()));
    };
    let count = values.get(count).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift shift-count value {} is unavailable", count.0))
    })?;
    let count_type = builder.func.dfg.value_type(count);
    let limit = builder.ins().iconst(count_type, i64::from(*width));
    let valid = builder.ins().icmp(IntCC::UnsignedLessThan, count, limit);
    let ok_block = builder.create_block();
    let trap_block = builder.create_block();
    builder.ins().brif(valid, ok_block, &[], trap_block, &[]);
    builder.switch_to_block(trap_block);
    emit_check_failure(module, builder, TrapCode::INTEGER_OVERFLOW, failure)?;
    builder.seal_block(trap_block);
    builder.switch_to_block(ok_block);
    builder.seal_block(ok_block);
    Ok(())
}

pub(in crate::codegen::cranelift) fn lower_trap_if_array_index_out_of_bounds(
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
    module: &mut ObjectModule,
    failure: Option<&str>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::TrapIfArrayIndexOutOfBounds { index, capacity, .. } = instruction
    else {
        return Err(BackendEmitError("Cranelift array-index guard expected".to_owned()));
    };
    let index = values.get(index).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift array-index value {} is unavailable", index.0))
    })?;
    let index_type = builder.func.dfg.value_type(index);
    let limit = builder.ins().iconst(index_type, i64::from(*capacity));
    let valid = builder.ins().icmp(IntCC::UnsignedLessThan, index, limit);
    let ok_block = builder.create_block();
    let trap_block = builder.create_block();
    builder.ins().brif(valid, ok_block, &[], trap_block, &[]);
    builder.switch_to_block(trap_block);
    emit_check_failure(module, builder, TrapCode::INTEGER_OVERFLOW, failure)?;
    builder.seal_block(trap_block);
    builder.switch_to_block(ok_block);
    builder.seal_block(ok_block);
    Ok(())
}

pub(in crate::codegen::cranelift) fn lower_trap_if_signed_division_overflow(
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
    module: &mut ObjectModule,
    failure: Option<&str>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::TrapIfSignedDivisionOverflow { left, right, width, .. } = instruction
    else {
        return Err(BackendEmitError("Cranelift signed-division guard expected".to_owned()));
    };
    let left = values.get(left).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift division left operand {} is unavailable", left.0))
    })?;
    let right = values.get(right).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift division right operand {} is unavailable", right.0))
    })?;
    let value_type = builder.func.dfg.value_type(left);
    let minimum = builder.ins().iconst(value_type, 1i64 << (u32::from(*width) - 1));
    let minus_one = builder.ins().iconst(value_type, -1);
    let minimum_match = builder.ins().icmp(IntCC::Equal, left, minimum);
    let minus_one_match = builder.ins().icmp(IntCC::Equal, right, minus_one);
    let overflow = builder.ins().band(minimum_match, minus_one_match);
    let ok_block = builder.create_block();
    let trap_block = builder.create_block();
    builder.ins().brif(overflow, trap_block, &[], ok_block, &[]);
    builder.switch_to_block(trap_block);
    emit_check_failure(module, builder, TrapCode::INTEGER_OVERFLOW, failure)?;
    builder.seal_block(trap_block);
    builder.switch_to_block(ok_block);
    builder.seal_block(ok_block);
    Ok(())
}

fn is_signed(ty: BackendValueType) -> bool {
    matches!(ty, BackendValueType::Integer { signed: true, .. })
}

pub(in crate::codegen::cranelift) fn lower_integer_compare(
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::IntegerCompare { result, operation, left, right, signed, .. } =
        instruction
    else {
        return Err(BackendEmitError("Cranelift integer comparison expected".to_owned()));
    };
    let left = values.get(left).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift comparison left operand {} is unavailable", left.0))
    })?;
    let right = values.get(right).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift comparison right operand {} is unavailable", right.0))
    })?;
    let right = coerce_integer_width(builder, right, builder.func.dfg.value_type(left));
    let condition = match (operation, signed) {
        (BackendIntegerCompareOp::LessThan, true) => IntCC::SignedLessThan,
        (BackendIntegerCompareOp::LessEquals, true) => IntCC::SignedLessThanOrEqual,
        (BackendIntegerCompareOp::GreaterThan, true) => IntCC::SignedGreaterThan,
        (BackendIntegerCompareOp::GreaterEquals, true) => IntCC::SignedGreaterThanOrEqual,
        (BackendIntegerCompareOp::Equals, _) => IntCC::Equal,
        (BackendIntegerCompareOp::NotEquals, _) => IntCC::NotEqual,
        (BackendIntegerCompareOp::LessThan, false) => IntCC::UnsignedLessThan,
        (BackendIntegerCompareOp::LessEquals, false) => IntCC::UnsignedLessThanOrEqual,
        (BackendIntegerCompareOp::GreaterThan, false) => IntCC::UnsignedGreaterThan,
        (BackendIntegerCompareOp::GreaterEquals, false) => IntCC::UnsignedGreaterThanOrEqual,
    };
    values.insert(*result, builder.ins().icmp(condition, left, right));
    Ok(())
}

pub(in crate::codegen::cranelift) fn lower_scalar_select(
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::ScalarSelect { result, condition, then_value, else_value, .. } =
        instruction
    else {
        return Err(BackendEmitError("Cranelift scalar select expected".to_owned()));
    };
    let condition = values.get(condition).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift select condition {} is unavailable", condition.0))
    })?;
    let then_value = values.get(then_value).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift select then value {} is unavailable", then_value.0))
    })?;
    let else_value = values.get(else_value).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift select else value {} is unavailable", else_value.0))
    })?;
    values.insert(*result, builder.ins().select(condition, then_value, else_value));
    Ok(())
}

fn coerce_integer_width(
    builder: &mut FunctionBuilder<'_>,
    value: cranelift_codegen::ir::Value,
    target: cranelift_codegen::ir::Type,
) -> cranelift_codegen::ir::Value {
    let source = builder.func.dfg.value_type(value);
    if source == target {
        return value;
    }
    if source.bits() < target.bits() {
        builder.ins().uextend(target, value)
    } else {
        builder.ins().ireduce(target, value)
    }
}

pub(in crate::codegen::cranelift) fn lower_integer_unary(
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
    module: &mut ObjectModule,
    failure: Option<&str>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::IntegerUnary { result, operation, operand, .. } = instruction else {
        return Err(BackendEmitError("Cranelift integer unary operation expected".to_owned()));
    };
    let operand = values.get(operand).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift unary operand {} is unavailable", operand.0))
    })?;
    let value = match operation {
        BackendIntegerUnaryOp::Negate => builder.ins().ineg(operand),
        BackendIntegerUnaryOp::TrappingNegate => {
            let ty = builder.func.dfg.value_type(operand);
            let zero = builder.ins().iconst(ty, 0);
            let signed = matches!(
                instruction,
                BackendInstruction::IntegerUnary {
                    ty: BackendValueType::Integer { signed: true, .. },
                    ..
                }
            );
            let (value, overflow) = overflowing(
                builder,
                BackendIntegerBinaryOp::TrappingSubtract,
                signed,
                zero,
                operand,
            );
            check_failed_if(module, builder, overflow, TrapCode::INTEGER_OVERFLOW, failure)?;
            value
        }
        BackendIntegerUnaryOp::LogicalNot => {
            let value_type = builder.func.dfg.value_type(operand);
            let zero = builder.ins().iconst(value_type, 0);
            builder.ins().icmp(IntCC::Equal, operand, zero)
        }
        BackendIntegerUnaryOp::BitwiseNot => builder.ins().bnot(operand),
    };
    values.insert(*result, value);
    Ok(())
}
