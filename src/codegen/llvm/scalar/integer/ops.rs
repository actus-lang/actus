use std::collections::BTreeMap;
use std::ffi::CStr;

use llvm_sys::LLVMIntPredicate;
use llvm_sys::core::{
    LLVMBuildAShr, LLVMBuildAdd, LLVMBuildAnd, LLVMBuildICmp, LLVMBuildLShr, LLVMBuildMul,
    LLVMBuildOr, LLVMBuildSDiv, LLVMBuildSExt, LLVMBuildSRem, LLVMBuildSelect, LLVMBuildShl,
    LLVMBuildSub, LLVMBuildUDiv, LLVMBuildURem, LLVMBuildXor, LLVMBuildZExt, LLVMGetIntTypeWidth,
    LLVMTypeOf,
};
use llvm_sys::prelude::{LLVMBuilderRef, LLVMValueRef};

use super::super::super::super::{
    BackendEmitError, BackendInstruction, BackendIntegerBinaryOp, BackendIntegerCompareOp,
    BackendIntegerUnaryOp, BackendValueId,
};
use llvm_sys::core::{LLVMConstAllOnes, LLVMConstInt, LLVMConstNull};

pub(in crate::codegen::llvm) fn emit_integer_binary(
    site: &super::overflow::OverflowSite,
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
    let signed =
        matches!(ty, super::super::super::super::BackendValueType::Integer { signed: true, .. });
    let builder = site.builder;
    let (left, right) = normalize_integer_pair(builder, left, right, signed)?;
    let value =
        match super::overflow::emit_checked_arithmetic(site, *operation, signed, left, right) {
            Some(value) => value?,
            None => build_integer_binary(builder, *operation, *ty, left, right),
        };
    if value.is_null() {
        return Err(BackendEmitError("LLVM failed to emit integer binary operation".to_owned()));
    }
    let value = if matches!(ty, super::super::super::super::BackendValueType::Bool) {
        unsafe {
            LLVMBuildICmp(
                builder,
                LLVMIntPredicate::LLVMIntNE,
                value,
                LLVMConstNull(LLVMTypeOf(value)),
                c"actus.logical.bool".as_ptr(),
            )
        }
    } else {
        value
    };
    values.insert(*result, value);
    Ok(false)
}

fn build_integer_binary(
    builder: LLVMBuilderRef,
    operation: BackendIntegerBinaryOp,
    ty: super::super::super::super::BackendValueType,
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
            BackendIntegerBinaryOp::Divide | BackendIntegerBinaryOp::Remainder
                if matches!(
                    ty,
                    super::super::super::super::BackendValueType::Integer { signed: true, .. }
                ) =>
            {
                build_signed_division_or_remainder(builder, operation, left, right)
            }
            BackendIntegerBinaryOp::Divide | BackendIntegerBinaryOp::Remainder => {
                build_unsigned_division_or_remainder(builder, operation, left, right)
            }
            BackendIntegerBinaryOp::ShiftLeft => {
                LLVMBuildShl(builder, left, right, c"actus.shl".as_ptr())
            }
            BackendIntegerBinaryOp::ShiftRight
                if matches!(
                    ty,
                    super::super::super::super::BackendValueType::Integer { signed: true, .. }
                ) =>
            {
                LLVMBuildAShr(builder, left, right, c"actus.ashr".as_ptr())
            }
            BackendIntegerBinaryOp::ShiftRight => {
                LLVMBuildLShr(builder, left, right, c"actus.lshr".as_ptr())
            }
            BackendIntegerBinaryOp::BitwiseAnd
            | BackendIntegerBinaryOp::BitwiseOr
            | BackendIntegerBinaryOp::BitwiseXor
            | BackendIntegerBinaryOp::LogicalAnd
            | BackendIntegerBinaryOp::LogicalOr => build_bitwise(builder, operation, left, right),
            BackendIntegerBinaryOp::TrappingAdd
            | BackendIntegerBinaryOp::TrappingSubtract
            | BackendIntegerBinaryOp::TrappingMultiply
            | BackendIntegerBinaryOp::SaturatingAdd
            | BackendIntegerBinaryOp::SaturatingSubtract
            | BackendIntegerBinaryOp::SaturatingMultiply => std::ptr::null_mut(),
        }
    }
}

fn build_bitwise(
    builder: LLVMBuilderRef,
    operation: BackendIntegerBinaryOp,
    left: LLVMValueRef,
    right: LLVMValueRef,
) -> LLVMValueRef {
    unsafe {
        match operation {
            BackendIntegerBinaryOp::BitwiseAnd | BackendIntegerBinaryOp::LogicalAnd => {
                LLVMBuildAnd(builder, left, right, c"actus.and".as_ptr())
            }
            BackendIntegerBinaryOp::BitwiseOr | BackendIntegerBinaryOp::LogicalOr => {
                LLVMBuildOr(builder, left, right, c"actus.or".as_ptr())
            }
            BackendIntegerBinaryOp::BitwiseXor => {
                LLVMBuildXor(builder, left, right, c"actus.xor".as_ptr())
            }
            _ => unreachable!(),
        }
    }
}

fn build_signed_division_or_remainder(
    builder: LLVMBuilderRef,
    operation: BackendIntegerBinaryOp,
    left: LLVMValueRef,
    right: LLVMValueRef,
) -> LLVMValueRef {
    unsafe {
        match operation {
            BackendIntegerBinaryOp::Divide => {
                LLVMBuildSDiv(builder, left, right, c"actus.sdiv".as_ptr())
            }
            // `srem MIN, -1` is undefined behaviour in LLVM (a hardware trap
            // on x86), while the result is mathematically 0. A divisor of -1
            // is replaced with 1, which has the same remainder, so
            // `MIN % -1` is 0 on every backend and never traps.
            BackendIntegerBinaryOp::Remainder => {
                let ty = LLVMTypeOf(right);
                let is_minus_one = LLVMBuildICmp(
                    builder,
                    LLVMIntPredicate::LLVMIntEQ,
                    right,
                    LLVMConstAllOnes(ty),
                    c"actus.srem.negone".as_ptr(),
                );
                let divisor = LLVMBuildSelect(
                    builder,
                    is_minus_one,
                    LLVMConstInt(ty, 1, 0),
                    right,
                    c"actus.srem.divisor".as_ptr(),
                );
                LLVMBuildSRem(builder, left, divisor, c"actus.srem".as_ptr())
            }
            _ => unreachable!(),
        }
    }
}

fn build_unsigned_division_or_remainder(
    builder: LLVMBuilderRef,
    operation: BackendIntegerBinaryOp,
    left: LLVMValueRef,
    right: LLVMValueRef,
) -> LLVMValueRef {
    unsafe {
        match operation {
            BackendIntegerBinaryOp::Divide => {
                LLVMBuildUDiv(builder, left, right, c"actus.udiv".as_ptr())
            }
            BackendIntegerBinaryOp::Remainder => {
                LLVMBuildURem(builder, left, right, c"actus.urem".as_ptr())
            }
            _ => unreachable!(),
        }
    }
}

pub(in crate::codegen::llvm) fn emit_integer_compare(
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

pub(in crate::codegen::llvm) fn emit_integer_unary(
    site: &super::overflow::OverflowSite,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let builder = site.builder;
    let BackendInstruction::IntegerUnary { result, operation, operand, ty, .. } = instruction
    else {
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
            BackendIntegerUnaryOp::TrappingNegate => {
                let signed = matches!(
                    ty,
                    super::super::super::super::BackendValueType::Integer { signed: true, .. }
                );
                super::overflow::emit_trapping_negate(site, signed, operand)?
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
