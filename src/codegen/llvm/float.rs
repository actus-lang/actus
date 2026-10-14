use std::collections::BTreeMap;

use llvm_sys::LLVMRealPredicate;
use llvm_sys::core::{LLVMBuildFCmp, LLVMBuildFPExt, LLVMBuildFPTrunc, LLVMConstReal};
use llvm_sys::prelude::{LLVMBuilderRef, LLVMContextRef, LLVMValueRef};

use super::super::{
    BackendEmitError, BackendInstruction, BackendIntegerCompareOp, BackendValueId, BackendValueType,
};

pub(super) fn emit_float_constant(
    context: LLVMContextRef,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let BackendInstruction::FloatConstant { result, ty, bits, .. } = instruction else {
        return Err(BackendEmitError("LLVM float constant expected".to_owned()));
    };
    let (llvm_type, value) = match ty {
        BackendValueType::Float { width: 32 } => (
            unsafe { llvm_sys::core::LLVMFloatTypeInContext(context) },
            f32::from_bits(*bits as u32) as f64,
        ),
        BackendValueType::Float { width: 64 } => {
            (unsafe { llvm_sys::core::LLVMDoubleTypeInContext(context) }, f64::from_bits(*bits))
        }
        _ => return Err(BackendEmitError("LLVM float constant requires f32 or f64".to_owned())),
    };
    let constant = unsafe { LLVMConstReal(llvm_type, value) };
    if constant.is_null() {
        return Err(BackendEmitError("LLVM failed to emit float constant".to_owned()));
    }
    values.insert(*result, constant);
    Ok(false)
}

pub(super) fn emit_float_compare(
    builder: LLVMBuilderRef,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let BackendInstruction::FloatCompare { result, operation, left, right, ty, .. } = instruction
    else {
        return Err(BackendEmitError("LLVM float comparison expected".to_owned()));
    };
    if !matches!(ty, BackendValueType::Float { width: 32 | 64 }) {
        return Err(BackendEmitError("LLVM float comparison requires f32 or f64".to_owned()));
    }
    let left = values.get(left).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM float comparison left operand {} is unavailable", left.0))
    })?;
    let right = values.get(right).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM float comparison right operand {} is unavailable", right.0))
    })?;
    let predicate = match operation {
        BackendIntegerCompareOp::Equals => LLVMRealPredicate::LLVMRealOEQ,
        BackendIntegerCompareOp::NotEquals => LLVMRealPredicate::LLVMRealONE,
        BackendIntegerCompareOp::LessThan => LLVMRealPredicate::LLVMRealOLT,
        BackendIntegerCompareOp::LessEquals => LLVMRealPredicate::LLVMRealOLE,
        BackendIntegerCompareOp::GreaterThan => LLVMRealPredicate::LLVMRealOGT,
        BackendIntegerCompareOp::GreaterEquals => LLVMRealPredicate::LLVMRealOGE,
    };
    let value =
        unsafe { LLVMBuildFCmp(builder, predicate, left, right, c"actus.float.compare".as_ptr()) };
    if value.is_null() {
        return Err(BackendEmitError("LLVM failed to emit float comparison".to_owned()));
    }
    values.insert(*result, value);
    Ok(false)
}

pub(super) fn emit_float_cast(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let BackendInstruction::FloatCast { result, source, source_type, target_type, .. } =
        instruction
    else {
        return Err(BackendEmitError("LLVM float cast expected".to_owned()));
    };
    let source = values.get(source).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM float cast source {} is unavailable", source.0))
    })?;
    let (source_width, target_width) = match (source_type, target_type) {
        (BackendValueType::Float { width: source }, BackendValueType::Float { width: target }) => {
            (*source, *target)
        }
        _ => return Err(BackendEmitError("LLVM float cast requires f32 or f64".to_owned())),
    };
    let target = match target_width {
        32 => unsafe { llvm_sys::core::LLVMFloatTypeInContext(context) },
        64 => unsafe { llvm_sys::core::LLVMDoubleTypeInContext(context) },
        _ => return Err(BackendEmitError("LLVM float cast width is invalid".to_owned())),
    };
    let value = if source_width == target_width {
        source
    } else if target_width == 64 {
        unsafe { LLVMBuildFPExt(builder, source, target, c"actus.float.extend".as_ptr()) }
    } else {
        unsafe { LLVMBuildFPTrunc(builder, source, target, c"actus.float.truncate".as_ptr()) }
    };
    if value.is_null() {
        return Err(BackendEmitError("LLVM failed to emit float cast".to_owned()));
    }
    values.insert(*result, value);
    Ok(false)
}
