use std::collections::BTreeMap;

use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{InstBuilder, TrapCode, types};
use cranelift_frontend::FunctionBuilder;

use crate::codegen::{
    BackendEmitError, BackendInstruction, BackendIntegerBinaryOp, BackendIntegerCompareOp,
    BackendIntegerUnaryOp, BackendValueId, BackendValueType,
};

pub(super) fn lower_integer_binary(
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
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
    let value = match operation {
        BackendIntegerBinaryOp::Add => builder.ins().iadd(left, right),
        BackendIntegerBinaryOp::Subtract => builder.ins().isub(left, right),
        BackendIntegerBinaryOp::Multiply => builder.ins().imul(left, right),
        BackendIntegerBinaryOp::Divide => {
            if is_signed(*ty) {
                builder.ins().sdiv(left, right)
            } else {
                builder.ins().udiv(left, right)
            }
        }
        BackendIntegerBinaryOp::Remainder => {
            if is_signed(*ty) {
                builder.ins().srem(left, right)
            } else {
                builder.ins().urem(left, right)
            }
        }
        BackendIntegerBinaryOp::BitwiseAnd => builder.ins().band(left, right),
        BackendIntegerBinaryOp::BitwiseOr => builder.ins().bor(left, right),
        BackendIntegerBinaryOp::BitwiseXor => builder.ins().bxor(left, right),
    };
    values.insert(*result, value);
    Ok(())
}

pub(super) fn lower_trap_if_zero(
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
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
    builder.ins().trap(TrapCode::INTEGER_DIVISION_BY_ZERO);
    builder.seal_block(trap_block);
    builder.switch_to_block(ok_block);
    builder.seal_block(ok_block);
    Ok(())
}

fn is_signed(ty: BackendValueType) -> bool {
    matches!(ty, BackendValueType::Integer { signed: true, .. })
}

pub(super) fn lower_integer_compare(
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

pub(super) fn lower_integer_unary(
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::IntegerUnary { result, operation, operand, ty, .. } = instruction
    else {
        return Err(BackendEmitError("Cranelift integer unary operation expected".to_owned()));
    };
    let operand = values.get(operand).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift unary operand {} is unavailable", operand.0))
    })?;
    let operand_type = match ty {
        BackendValueType::Integer { width: 8, .. } | BackendValueType::Bool => types::I8,
        BackendValueType::Integer { width: 16, .. } => types::I16,
        BackendValueType::Integer { width: 32, .. } => types::I32,
        BackendValueType::Integer { width: 64, .. } => types::I64,
        _ => return Err(BackendEmitError("Cranelift unary operand must be an integer".to_owned())),
    };
    let value = match operation {
        BackendIntegerUnaryOp::Negate => builder.ins().ineg(operand),
        BackendIntegerUnaryOp::LogicalNot => {
            let zero = builder.ins().iconst(operand_type, 0);
            builder.ins().icmp(IntCC::Equal, operand, zero)
        }
        BackendIntegerUnaryOp::BitwiseNot => builder.ins().bnot(operand),
    };
    values.insert(*result, value);
    Ok(())
}
