use std::collections::BTreeMap;

use llvm_sys::LLVMTypeKind::LLVMPointerTypeKind;
use llvm_sys::core::{
    LLVMBuildMemMove, LLVMConstInt, LLVMGetTypeKind, LLVMInt64TypeInContext, LLVMTypeOf,
};
use llvm_sys::prelude::{LLVMBuilderRef, LLVMValueRef};

use super::super::{BackendEmitError, BackendValueId};

pub(super) fn emit_memory_copy(
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    destination_id: BackendValueId,
    source_id: BackendValueId,
    size: u32,
    alignment: u32,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let destination = pointer_value(destination_id, values, "destination")?;
    let source = pointer_value(source_id, values, "source")?;
    let length = unsafe { LLVMConstInt(LLVMInt64TypeInContext(context), u64::from(size), 0) };
    let copy =
        unsafe { LLVMBuildMemMove(builder, destination, alignment, source, alignment, length) };
    (!copy.is_null())
        .then_some(false)
        .ok_or_else(|| BackendEmitError("LLVM failed to build memory copy".to_owned()))
}

fn pointer_value(
    id: BackendValueId,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
    role: &str,
) -> Result<LLVMValueRef, BackendEmitError> {
    let value = values.get(&id).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM memory-copy {role} value {} is unavailable", id.0))
    })?;
    let kind = unsafe { LLVMGetTypeKind(LLVMTypeOf(value)) };
    (kind == LLVMPointerTypeKind)
        .then_some(value)
        .ok_or_else(|| BackendEmitError(format!("LLVM memory-copy {role} must be a pointer")))
}
