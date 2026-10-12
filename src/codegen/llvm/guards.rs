use std::collections::BTreeMap;

use llvm_sys::LLVMIntPredicate;
use llvm_sys::core::{
    LLVMAddFunction, LLVMAppendBasicBlockInContext, LLVMBuildAnd, LLVMBuildCall2, LLVMBuildCondBr,
    LLVMBuildICmp, LLVMBuildUnreachable, LLVMConstInt, LLVMConstNull, LLVMFunctionType,
    LLVMGetGlobalParent, LLVMGetNamedFunction, LLVMInt32TypeInContext, LLVMPositionBuilderAtEnd,
    LLVMTypeOf,
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
            c"actus.divisor.nonzero".as_ptr(),
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

fn emit_llvm_trap(
    context: llvm_sys::prelude::LLVMContextRef,
    function: LLVMValueRef,
    builder: LLVMBuilderRef,
) -> Result<(), BackendEmitError> {
    unsafe {
        let module = LLVMGetGlobalParent(function);
        if module.is_null() {
            return Err(BackendEmitError("LLVM trap function has no parent module".to_owned()));
        }
        let name = b"raise\0";
        let trap = {
            let existing = LLVMGetNamedFunction(module, name.as_ptr().cast());
            if existing.is_null() {
                let mut parameter_types = [LLVMInt32TypeInContext(context)];
                let trap_type = LLVMFunctionType(
                    LLVMInt32TypeInContext(context),
                    parameter_types.as_mut_ptr(),
                    1,
                    0,
                );
                LLVMAddFunction(module, name.as_ptr().cast(), trap_type)
            } else {
                existing
            }
        };
        if trap.is_null() {
            return Err(BackendEmitError("LLVM failed to declare raise function".to_owned()));
        }
        let mut arguments = [LLVMConstInt(LLVMInt32TypeInContext(context), 4, 0)];
        let mut parameter_types = [LLVMInt32TypeInContext(context)];
        let trap_type =
            LLVMFunctionType(LLVMInt32TypeInContext(context), parameter_types.as_mut_ptr(), 1, 0);
        if LLVMBuildCall2(builder, trap_type, trap, arguments.as_mut_ptr(), 1, c"".as_ptr())
            .is_null()
        {
            return Err(BackendEmitError("LLVM failed to emit raise call".to_owned()));
        }
        if LLVMBuildUnreachable(builder).is_null() {
            return Err(BackendEmitError("LLVM failed to terminate trap block".to_owned()));
        }
    }
    Ok(())
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
            c"actus.shift.valid".as_ptr(),
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
        emit_llvm_trap(context, function, builder)?;
        LLVMPositionBuilderAtEnd(builder, ok_block);
    }
    Ok(false)
}

pub(super) fn emit_trap_if_array_index_out_of_bounds(
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    function: LLVMValueRef,
    instruction: &BackendInstruction,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let BackendInstruction::TrapIfArrayIndexOutOfBounds { index, capacity, span, .. } = instruction
    else {
        return Err(BackendEmitError("LLVM array-index guard expected".to_owned()));
    };
    let index = guard_value(values, *index, "array-index value")?;
    let limit = unsafe { LLVMConstInt(LLVMTypeOf(index), u64::from(*capacity), 0) };
    let condition = unsafe {
        LLVMBuildICmp(
            builder,
            LLVMIntPredicate::LLVMIntULT,
            index,
            limit,
            c"actus.array.index.valid".as_ptr(),
        )
    };
    if condition.is_null() {
        return Err(BackendEmitError("LLVM failed to emit array-index guard".to_owned()));
    }
    let (ok_block, trap_block) = append_guard_blocks(context, function, "array.index", *span)?;
    if unsafe { LLVMBuildCondBr(builder, condition, ok_block, trap_block) }.is_null() {
        return Err(BackendEmitError("LLVM failed to emit array-index guard branch".to_owned()));
    }
    unsafe {
        LLVMPositionBuilderAtEnd(builder, trap_block);
        emit_llvm_trap(context, function, builder)?;
        LLVMPositionBuilderAtEnd(builder, ok_block);
    }
    Ok(false)
}

pub(super) fn emit_trap_if_signed_division_overflow(
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    function: LLVMValueRef,
    instruction: &BackendInstruction,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let BackendInstruction::TrapIfSignedDivisionOverflow { left, right, width, span, .. } =
        instruction
    else {
        return Err(BackendEmitError("LLVM signed-division guard expected".to_owned()));
    };
    let left = guard_value(values, *left, "division left operand")?;
    let right = guard_value(values, *right, "division right operand")?;
    let ty = unsafe { LLVMTypeOf(left) };
    let minimum = unsafe { LLVMConstInt(ty, 1u64 << (u32::from(*width) - 1), 0) };
    let minus_one = unsafe { LLVMConstInt(ty, u64::MAX, 0) };
    let minimum_match = unsafe {
        LLVMBuildICmp(
            builder,
            LLVMIntPredicate::LLVMIntEQ,
            left,
            minimum,
            c"actus.div.min".as_ptr(),
        )
    };
    let minus_one_match = unsafe {
        LLVMBuildICmp(
            builder,
            LLVMIntPredicate::LLVMIntEQ,
            right,
            minus_one,
            c"actus.div.negone".as_ptr(),
        )
    };
    let overflow = unsafe {
        LLVMBuildAnd(builder, minimum_match, minus_one_match, c"actus.div.overflow".as_ptr())
    };
    if minimum_match.is_null() || minus_one_match.is_null() || overflow.is_null() {
        return Err(BackendEmitError("LLVM failed to emit signed-division guard".to_owned()));
    }
    let (ok_block, trap_block) = append_guard_blocks(context, function, "division", *span)?;
    if unsafe { LLVMBuildCondBr(builder, overflow, trap_block, ok_block) }.is_null() {
        return Err(BackendEmitError("LLVM failed to emit division guard branch".to_owned()));
    }
    unsafe {
        LLVMPositionBuilderAtEnd(builder, trap_block);
        emit_llvm_trap(context, function, builder)?;
        LLVMPositionBuilderAtEnd(builder, ok_block);
    }
    Ok(false)
}

fn append_guard_blocks(
    context: llvm_sys::prelude::LLVMContextRef,
    function: LLVMValueRef,
    kind: &str,
    span: crate::lexer::SourceSpan,
) -> Result<
    (llvm_sys::prelude::LLVMBasicBlockRef, llvm_sys::prelude::LLVMBasicBlockRef),
    BackendEmitError,
> {
    let suffix = format!("{}.{}", span.start, span.end);
    let ok_name = std::ffi::CString::new(format!("actus.{kind}.ok.{suffix}"))
        .map_err(|_| BackendEmitError("LLVM guard block name contains NUL".to_owned()))?;
    let trap_name = std::ffi::CString::new(format!("actus.{kind}.trap.{suffix}"))
        .map_err(|_| BackendEmitError("LLVM guard block name contains NUL".to_owned()))?;
    let ok_block = unsafe { LLVMAppendBasicBlockInContext(context, function, ok_name.as_ptr()) };
    let trap_block =
        unsafe { LLVMAppendBasicBlockInContext(context, function, trap_name.as_ptr()) };
    if ok_block.is_null() || trap_block.is_null() {
        return Err(BackendEmitError("LLVM failed to create guard blocks".to_owned()));
    }
    Ok((ok_block, trap_block))
}

fn guard_value(
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
    id: BackendValueId,
    role: &str,
) -> Result<LLVMValueRef, BackendEmitError> {
    values
        .get(&id)
        .copied()
        .ok_or_else(|| BackendEmitError(format!("LLVM {role} {} is unavailable", id.0)))
}
