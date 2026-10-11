use std::collections::BTreeMap;

use llvm_sys::LLVMIntPredicate;
use llvm_sys::core::{
    LLVMAppendBasicBlockInContext, LLVMBuildAnd, LLVMBuildCondBr, LLVMBuildICmp, LLVMBuildSExt,
    LLVMBuildTrunc, LLVMBuildUnreachable, LLVMBuildZExt, LLVMConstInt, LLVMPositionBuilderAtEnd,
};
use llvm_sys::prelude::{LLVMBasicBlockRef, LLVMBuilderRef, LLVMValueRef};

use super::super::{BackendEmitError, BackendInstruction, BackendValueId, BackendValueType};

pub(super) fn emit_integer_cast(
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    function: LLVMValueRef,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let BackendInstruction::IntegerCast { result, source, source_type, target_type, span } =
        instruction
    else {
        return Err(BackendEmitError("LLVM integer cast expected".to_owned()));
    };
    let source_value = values
        .get(source)
        .copied()
        .ok_or_else(|| BackendEmitError(format!("LLVM cast source {} is unavailable", source.0)))?;
    let wide = widen_value(context, builder, source_value, *source_type)?;
    let valid = validity(context, builder, wide, *source_type, *target_type)?;
    let (ok_block, trap_block) = append_blocks(context, function, *span)?;
    if unsafe { LLVMBuildCondBr(builder, valid, ok_block, trap_block) }.is_null() {
        return Err(BackendEmitError("LLVM failed to emit integer cast branch".to_owned()));
    }
    unsafe {
        LLVMPositionBuilderAtEnd(builder, trap_block);
        LLVMBuildUnreachable(builder);
        LLVMPositionBuilderAtEnd(builder, ok_block);
    }
    let converted = convert_value(context, builder, wide, *target_type)?;
    values.insert(*result, converted);
    Ok(false)
}

fn widen_value(
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    value: LLVMValueRef,
    ty: BackendValueType,
) -> Result<LLVMValueRef, BackendEmitError> {
    let BackendValueType::Integer { signed, width } = ty else {
        return Err(BackendEmitError("LLVM cast source must be an integer".to_owned()));
    };
    if width == 64 {
        return Ok(value);
    }
    let value = unsafe {
        if signed {
            LLVMBuildSExt(builder, value, i64_type(context), b"actus.cast.sext\0".as_ptr().cast())
        } else {
            LLVMBuildZExt(builder, value, i64_type(context), b"actus.cast.zext\0".as_ptr().cast())
        }
    };
    (!value.is_null())
        .then_some(value)
        .ok_or_else(|| BackendEmitError("LLVM failed to widen integer cast source".to_owned()))
}

fn validity(
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    value: LLVMValueRef,
    source: BackendValueType,
    target: BackendValueType,
) -> Result<LLVMValueRef, BackendEmitError> {
    let (
        BackendValueType::Integer { signed: source_signed, .. },
        BackendValueType::Integer { signed: target_signed, width: target_width },
    ) = (source, target)
    else {
        return Err(BackendEmitError("LLVM cast types are invalid".to_owned()));
    };
    let zero = unsafe { LLVMConstInt(i64_type(context), 0, 0) };
    let lower = if source_signed && !target_signed {
        unsafe {
            LLVMBuildICmp(
                builder,
                LLVMIntPredicate::LLVMIntSGE,
                value,
                zero,
                b"actus.cast.lower\0".as_ptr().cast(),
            )
        }
    } else {
        unsafe { LLVMConstInt(bool_type(context), 1, 0) }
    };
    let upper = upper_bound(context, builder, value, source_signed, target_signed, target_width);
    if lower.is_null() || upper.is_null() {
        return Err(BackendEmitError("LLVM failed to emit integer cast bounds".to_owned()));
    }
    let valid =
        unsafe { LLVMBuildAnd(builder, lower, upper, b"actus.cast.valid\0".as_ptr().cast()) };
    (!valid.is_null())
        .then_some(valid)
        .ok_or_else(|| BackendEmitError("LLVM failed to combine integer cast bounds".to_owned()))
}

fn upper_bound(
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    value: LLVMValueRef,
    source_signed: bool,
    target_signed: bool,
    target_width: u8,
) -> LLVMValueRef {
    if !target_signed && target_width == 64 && !source_signed {
        return unsafe { LLVMConstInt(bool_type(context), 1, 0) };
    }
    let maximum = if target_signed {
        if target_width == 64 { i64::MAX } else { (1i64 << (target_width - 1)) - 1 }
    } else if target_width == 64 {
        i64::MAX
    } else {
        (1i64 << target_width) - 1
    };
    unsafe {
        LLVMBuildICmp(
            builder,
            if source_signed { LLVMIntPredicate::LLVMIntSLE } else { LLVMIntPredicate::LLVMIntULE },
            value,
            LLVMConstInt(i64_type(context), maximum as u64, 0),
            b"actus.cast.upper\0".as_ptr().cast(),
        )
    }
}

fn convert_value(
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    value: LLVMValueRef,
    target: BackendValueType,
) -> Result<LLVMValueRef, BackendEmitError> {
    let BackendValueType::Integer { width, .. } = target else {
        return Err(BackendEmitError("LLVM cast target must be an integer".to_owned()));
    };
    if width == 64 {
        return Ok(value);
    }
    let converted = unsafe {
        LLVMBuildTrunc(
            builder,
            value,
            integer_type(context, width),
            b"actus.cast.trunc\0".as_ptr().cast(),
        )
    };
    (!converted.is_null())
        .then_some(converted)
        .ok_or_else(|| BackendEmitError("LLVM failed to narrow integer cast result".to_owned()))
}

fn append_blocks(
    context: llvm_sys::prelude::LLVMContextRef,
    function: LLVMValueRef,
    span: crate::lexer::SourceSpan,
) -> Result<(LLVMBasicBlockRef, LLVMBasicBlockRef), BackendEmitError> {
    let suffix = format!("{}.{}", span.start, span.end);
    let ok_name = std::ffi::CString::new(format!("actus.cast.ok.{suffix}"))
        .map_err(|_| BackendEmitError("LLVM cast block name contains NUL".to_owned()))?;
    let trap_name = std::ffi::CString::new(format!("actus.cast.trap.{suffix}"))
        .map_err(|_| BackendEmitError("LLVM cast block name contains NUL".to_owned()))?;
    let ok = unsafe { LLVMAppendBasicBlockInContext(context, function, ok_name.as_ptr()) };
    let trap = unsafe { LLVMAppendBasicBlockInContext(context, function, trap_name.as_ptr()) };
    if ok.is_null() || trap.is_null() {
        return Err(BackendEmitError("LLVM failed to create integer cast blocks".to_owned()));
    }
    Ok((ok, trap))
}

fn i64_type(context: llvm_sys::prelude::LLVMContextRef) -> llvm_sys::prelude::LLVMTypeRef {
    unsafe { llvm_sys::core::LLVMInt64TypeInContext(context) }
}

fn bool_type(context: llvm_sys::prelude::LLVMContextRef) -> llvm_sys::prelude::LLVMTypeRef {
    unsafe { llvm_sys::core::LLVMInt1TypeInContext(context) }
}

fn integer_type(
    context: llvm_sys::prelude::LLVMContextRef,
    width: u8,
) -> llvm_sys::prelude::LLVMTypeRef {
    unsafe {
        match width {
            8 => llvm_sys::core::LLVMInt8TypeInContext(context),
            16 => llvm_sys::core::LLVMInt16TypeInContext(context),
            32 => llvm_sys::core::LLVMInt32TypeInContext(context),
            _ => llvm_sys::core::LLVMInt64TypeInContext(context),
        }
    }
}
