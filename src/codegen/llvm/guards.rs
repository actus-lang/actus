use std::collections::BTreeMap;

use llvm_sys::LLVMIntPredicate;
use llvm_sys::core::{
    LLVMAppendBasicBlockInContext, LLVMBuildCondBr, LLVMBuildICmp, LLVMBuildUnreachable,
    LLVMConstInt, LLVMConstNull, LLVMPositionBuilderAtEnd, LLVMTypeOf,
};
use llvm_sys::prelude::{LLVMBuilderRef, LLVMValueRef};

use super::super::{BackendEmitError, BackendInstruction, BackendValueId};

pub(super) fn emit_trap_if_zero(
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    function: LLVMValueRef,
    instruction: &BackendInstruction,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let BackendInstruction::TrapIfZero { value, span, .. } = instruction else {
        return Err(BackendEmitError("LLVM zero guard expected".to_owned()));
    };
    let value = values.get(value).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM zero-guard value {} is unavailable", value.0))
    })?;
    let zero = unsafe { LLVMConstNull(LLVMTypeOf(value)) };
    let condition = unsafe {
        LLVMBuildICmp(
            builder,
            LLVMIntPredicate::LLVMIntNE,
            value,
            zero,
            b"actus.divisor.nonzero\0".as_ptr().cast(),
        )
    };
    if condition.is_null() {
        return Err(BackendEmitError("LLVM failed to emit zero guard comparison".to_owned()));
    }
    let suffix = format!("{}.{}", span.start, span.end);
    let ok_name = std::ffi::CString::new(format!("actus.divisor.ok.{suffix}"))
        .map_err(|_| BackendEmitError("LLVM zero-guard block name contains NUL".to_owned()))?;
    let trap_name = std::ffi::CString::new(format!("actus.divisor.trap.{suffix}"))
        .map_err(|_| BackendEmitError("LLVM zero-guard block name contains NUL".to_owned()))?;
    let ok_block = unsafe { LLVMAppendBasicBlockInContext(context, function, ok_name.as_ptr()) };
    let trap_block =
        unsafe { LLVMAppendBasicBlockInContext(context, function, trap_name.as_ptr()) };
    if ok_block.is_null() || trap_block.is_null() {
        return Err(BackendEmitError("LLVM failed to create zero-guard blocks".to_owned()));
    }
    if unsafe { LLVMBuildCondBr(builder, condition, ok_block, trap_block) }.is_null() {
        return Err(BackendEmitError("LLVM failed to emit zero-guard branch".to_owned()));
    }
    unsafe {
        LLVMPositionBuilderAtEnd(builder, trap_block);
        LLVMBuildUnreachable(builder);
        LLVMPositionBuilderAtEnd(builder, ok_block);
    }
    Ok(false)
}

pub(super) fn emit_trap_if_shift_count_out_of_range(
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    function: LLVMValueRef,
    instruction: &BackendInstruction,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let BackendInstruction::TrapIfShiftCountOutOfRange { count, width, span, .. } = instruction
    else {
        return Err(BackendEmitError("LLVM shift-count guard expected".to_owned()));
    };
    let count = values.get(count).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM shift-count value {} is unavailable", count.0))
    })?;
    let limit = unsafe { LLVMConstInt(LLVMTypeOf(count), u64::from(*width), 0) };
    let condition = unsafe {
        LLVMBuildICmp(
            builder,
            LLVMIntPredicate::LLVMIntULT,
            count,
            limit,
            b"actus.shift.valid\0".as_ptr().cast(),
        )
    };
    if condition.is_null() {
        return Err(BackendEmitError("LLVM failed to emit shift-count guard".to_owned()));
    }
    let suffix = format!("{}.{}", span.start, span.end);
    let ok_name = std::ffi::CString::new(format!("actus.shift.ok.{suffix}"))
        .map_err(|_| BackendEmitError("LLVM shift guard block name contains NUL".to_owned()))?;
    let trap_name = std::ffi::CString::new(format!("actus.shift.trap.{suffix}"))
        .map_err(|_| BackendEmitError("LLVM shift guard block name contains NUL".to_owned()))?;
    let ok_block = unsafe { LLVMAppendBasicBlockInContext(context, function, ok_name.as_ptr()) };
    let trap_block =
        unsafe { LLVMAppendBasicBlockInContext(context, function, trap_name.as_ptr()) };
    if ok_block.is_null() || trap_block.is_null() {
        return Err(BackendEmitError("LLVM failed to create shift guard blocks".to_owned()));
    }
    if unsafe { LLVMBuildCondBr(builder, condition, ok_block, trap_block) }.is_null() {
        return Err(BackendEmitError("LLVM failed to emit shift guard branch".to_owned()));
    }
    unsafe {
        LLVMPositionBuilderAtEnd(builder, trap_block);
        LLVMBuildUnreachable(builder);
        LLVMPositionBuilderAtEnd(builder, ok_block);
    }
    Ok(false)
}
