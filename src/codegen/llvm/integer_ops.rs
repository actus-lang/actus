use std::collections::BTreeMap;

use llvm_sys::LLVMIntPredicate;
use llvm_sys::core::{LLVMBuildAdd, LLVMBuildICmp, LLVMBuildMul, LLVMBuildSub, LLVMBuildXor};
use llvm_sys::prelude::{LLVMBuilderRef, LLVMValueRef};

use super::super::{
    BackendEmitError, BackendInstruction, BackendIntegerBinaryOp, BackendIntegerCompareOp,
    BackendIntegerUnaryOp, BackendValueId,
};
use llvm_sys::core::{LLVMConstAllOnes, LLVMConstNull, LLVMTypeOf};

pub(super) fn emit_integer_binary(
    builder: LLVMBuilderRef,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let BackendInstruction::IntegerBinary { result, operation, left, right, .. } = instruction
    else {
        return Err(BackendEmitError("LLVM integer operation expected".to_owned()));
    };
    let left = values.get(left).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM binary left operand {} is unavailable", left.0))
    })?;
    let right = values.get(right).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM binary right operand {} is unavailable", right.0))
    })?;
    let value = unsafe {
        match operation {
            BackendIntegerBinaryOp::Add => {
                LLVMBuildAdd(builder, left, right, b"actus.add\0".as_ptr().cast())
            }
            BackendIntegerBinaryOp::Subtract => {
                LLVMBuildSub(builder, left, right, b"actus.sub\0".as_ptr().cast())
            }
            BackendIntegerBinaryOp::Multiply => {
                LLVMBuildMul(builder, left, right, b"actus.mul\0".as_ptr().cast())
            }
        }
    };
    if value.is_null() {
        return Err(BackendEmitError("LLVM failed to emit integer binary operation".to_owned()));
    }
    values.insert(*result, value);
    Ok(false)
}

pub(super) fn emit_integer_compare(
    builder: LLVMBuilderRef,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let BackendInstruction::IntegerCompare { result, operation, left, right, signed, .. } =
        instruction
    else {
        return Err(BackendEmitError("LLVM integer comparison expected".to_owned()));
    };
    let left = values.get(left).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM comparison left operand {} is unavailable", left.0))
    })?;
    let right = values.get(right).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM comparison right operand {} is unavailable", right.0))
    })?;
    let predicate = match (operation, signed) {
        (BackendIntegerCompareOp::LessThan, true) => LLVMIntPredicate::LLVMIntSLT,
        (BackendIntegerCompareOp::LessEquals, true) => LLVMIntPredicate::LLVMIntSLE,
        (BackendIntegerCompareOp::GreaterThan, true) => LLVMIntPredicate::LLVMIntSGT,
        (BackendIntegerCompareOp::GreaterEquals, true) => LLVMIntPredicate::LLVMIntSGE,
        (BackendIntegerCompareOp::Equals, _) => LLVMIntPredicate::LLVMIntEQ,
        (BackendIntegerCompareOp::NotEquals, _) => LLVMIntPredicate::LLVMIntNE,
        (BackendIntegerCompareOp::LessThan, false) => LLVMIntPredicate::LLVMIntULT,
        (BackendIntegerCompareOp::LessEquals, false) => LLVMIntPredicate::LLVMIntULE,
        (BackendIntegerCompareOp::GreaterThan, false) => LLVMIntPredicate::LLVMIntUGT,
        (BackendIntegerCompareOp::GreaterEquals, false) => LLVMIntPredicate::LLVMIntUGE,
    };
    let value = unsafe {
        LLVMBuildICmp(builder, predicate, left, right, b"actus.compare\0".as_ptr().cast())
    };
    if value.is_null() {
        return Err(BackendEmitError("LLVM failed to emit integer comparison".to_owned()));
    }
    values.insert(*result, value);
    Ok(false)
}

pub(super) fn emit_integer_unary(
    builder: LLVMBuilderRef,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let BackendInstruction::IntegerUnary { result, operation, operand, .. } = instruction else {
        return Err(BackendEmitError("LLVM integer unary operation expected".to_owned()));
    };
    let operand = values.get(operand).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM unary operand {} is unavailable", operand.0))
    })?;
    let value = unsafe {
        match operation {
            BackendIntegerUnaryOp::Negate => {
                let zero = LLVMConstNull(LLVMTypeOf(operand));
                LLVMBuildSub(builder, zero, operand, b"actus.negate\0".as_ptr().cast())
            }
            BackendIntegerUnaryOp::LogicalNot => {
                let zero = LLVMConstNull(LLVMTypeOf(operand));
                LLVMBuildICmp(
                    builder,
                    LLVMIntPredicate::LLVMIntEQ,
                    operand,
                    zero,
                    b"actus.logical_not\0".as_ptr().cast(),
                )
            }
            BackendIntegerUnaryOp::BitwiseNot => {
                let all_ones = LLVMConstAllOnes(LLVMTypeOf(operand));
                LLVMBuildXor(builder, operand, all_ones, b"actus.bitwise_not\0".as_ptr().cast())
            }
        }
    };
    if value.is_null() {
        return Err(BackendEmitError("LLVM failed to emit integer unary operation".to_owned()));
    }
    values.insert(*result, value);
    Ok(false)
}
