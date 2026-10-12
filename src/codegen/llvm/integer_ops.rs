use std::collections::BTreeMap;
use std::ffi::CStr;

use llvm_sys::LLVMIntPredicate;
use llvm_sys::core::{
    LLVMBuildAShr, LLVMBuildAdd, LLVMBuildAnd, LLVMBuildICmp, LLVMBuildLShr, LLVMBuildMul,
    LLVMBuildOr, LLVMBuildSDiv, LLVMBuildSExt, LLVMBuildSRem, LLVMBuildShl, LLVMBuildSub,
    LLVMBuildUDiv, LLVMBuildURem, LLVMBuildXor, LLVMBuildZExt, LLVMGetIntTypeWidth, LLVMTypeOf,
};
use llvm_sys::prelude::{LLVMBuilderRef, LLVMValueRef};

use super::super::{
    BackendEmitError, BackendInstruction, BackendIntegerBinaryOp, BackendIntegerCompareOp,
    BackendIntegerUnaryOp, BackendValueId,
};
use llvm_sys::core::{LLVMConstAllOnes, LLVMConstNull};

pub(super) fn emit_integer_binary(
    builder: LLVMBuilderRef,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let BackendInstruction::IntegerBinary { result, operation, left, right, ty, .. } = instruction
    else {
        return Err(BackendEmitError("LLVM integer operation expected".to_owned()));
    };
    let left = values.get(left).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM binary left operand {} is unavailable", left.0))
    })?;
    let right = values.get(right).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM binary right operand {} is unavailable", right.0))
    })?;
    let signed = matches!(ty, super::super::BackendValueType::Integer { signed: true, .. });
    let (left, right) = normalize_integer_pair(builder, left, right, signed)?;
    let value = build_integer_binary(builder, *operation, *ty, left, right);
    if value.is_null() {
        return Err(BackendEmitError("LLVM failed to emit integer binary operation".to_owned()));
    }
    values.insert(*result, value);
    Ok(false)
}

fn build_integer_binary(
    builder: LLVMBuilderRef,
    operation: BackendIntegerBinaryOp,
    ty: super::super::BackendValueType,
    left: LLVMValueRef,
    right: LLVMValueRef,
) -> LLVMValueRef {
    unsafe {
        match operation {
            BackendIntegerBinaryOp::Add => {
                LLVMBuildAdd(builder, left, right, c"actus.add".as_ptr())
            }
            BackendIntegerBinaryOp::Subtract => {
                LLVMBuildSub(builder, left, right, c"actus.sub".as_ptr())
            }
            BackendIntegerBinaryOp::Multiply => {
                LLVMBuildMul(builder, left, right, c"actus.mul".as_ptr())
            }
            BackendIntegerBinaryOp::Divide
                if matches!(ty, super::super::BackendValueType::Integer { signed: true, .. }) =>
            {
                LLVMBuildSDiv(builder, left, right, c"actus.sdiv".as_ptr())
            }
            BackendIntegerBinaryOp::Divide => {
                LLVMBuildUDiv(builder, left, right, c"actus.udiv".as_ptr())
            }
            BackendIntegerBinaryOp::Remainder
                if matches!(ty, super::super::BackendValueType::Integer { signed: true, .. }) =>
            {
                LLVMBuildSRem(builder, left, right, c"actus.srem".as_ptr())
            }
            BackendIntegerBinaryOp::Remainder => {
                LLVMBuildURem(builder, left, right, c"actus.urem".as_ptr())
            }
            BackendIntegerBinaryOp::ShiftLeft => {
                LLVMBuildShl(builder, left, right, c"actus.shl".as_ptr())
            }
            BackendIntegerBinaryOp::ShiftRight
                if matches!(ty, super::super::BackendValueType::Integer { signed: true, .. }) =>
            {
                LLVMBuildAShr(builder, left, right, c"actus.ashr".as_ptr())
            }
            BackendIntegerBinaryOp::ShiftRight => {
                LLVMBuildLShr(builder, left, right, c"actus.lshr".as_ptr())
            }
            BackendIntegerBinaryOp::BitwiseAnd => {
                LLVMBuildAnd(builder, left, right, c"actus.and".as_ptr())
            }
            BackendIntegerBinaryOp::BitwiseOr => {
                LLVMBuildOr(builder, left, right, c"actus.or".as_ptr())
            }
            BackendIntegerBinaryOp::BitwiseXor => {
                LLVMBuildXor(builder, left, right, c"actus.xor".as_ptr())
            }
            BackendIntegerBinaryOp::LogicalAnd => {
                LLVMBuildAnd(builder, left, right, c"actus.logical_and".as_ptr())
            }
            BackendIntegerBinaryOp::LogicalOr => {
                LLVMBuildOr(builder, left, right, c"actus.logical_or".as_ptr())
            }
        }
    }
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
    let (left, right) = normalize_integer_pair(builder, left, right, *signed)?;
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
    let value =
        unsafe { LLVMBuildICmp(builder, predicate, left, right, c"actus.compare".as_ptr()) };
    if value.is_null() {
        return Err(BackendEmitError("LLVM failed to emit integer comparison".to_owned()));
    }
    values.insert(*result, value);
    Ok(false)
}

fn normalize_integer_pair(
    builder: LLVMBuilderRef,
    left: LLVMValueRef,
    right: LLVMValueRef,
    signed: bool,
) -> Result<(LLVMValueRef, LLVMValueRef), BackendEmitError> {
    let left_width = unsafe { LLVMGetIntTypeWidth(LLVMTypeOf(left)) };
    let right_width = unsafe { LLVMGetIntTypeWidth(LLVMTypeOf(right)) };
    if left_width == right_width {
        return Ok((left, right));
    }
    let cast = |value, target, name: &CStr| unsafe {
        if signed {
            LLVMBuildSExt(builder, value, target, name.as_ptr())
        } else {
            LLVMBuildZExt(builder, value, target, name.as_ptr())
        }
    };
    if left_width < right_width {
        let value = cast(left, unsafe { LLVMTypeOf(right) }, c"actus.int.sext.left");
        (!value.is_null())
            .then_some((value, right))
            .ok_or_else(|| BackendEmitError("LLVM integer operand extension failed".to_owned()))
    } else {
        let value = cast(right, unsafe { LLVMTypeOf(left) }, c"actus.int.sext.right");
        (!value.is_null())
            .then_some((left, value))
            .ok_or_else(|| BackendEmitError("LLVM integer operand extension failed".to_owned()))
    }
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
                LLVMBuildSub(builder, zero, operand, c"actus.negate".as_ptr())
            }
            BackendIntegerUnaryOp::LogicalNot => {
                let zero = LLVMConstNull(LLVMTypeOf(operand));
                LLVMBuildICmp(
                    builder,
                    LLVMIntPredicate::LLVMIntEQ,
                    operand,
                    zero,
                    c"actus.logical_not".as_ptr(),
                )
            }
            BackendIntegerUnaryOp::BitwiseNot => {
                let all_ones = LLVMConstAllOnes(LLVMTypeOf(operand));
                LLVMBuildXor(builder, operand, all_ones, c"actus.bitwise_not".as_ptr())
            }
        }
    };
    if value.is_null() {
        return Err(BackendEmitError("LLVM failed to emit integer unary operation".to_owned()));
    }
    values.insert(*result, value);
    Ok(false)
}
