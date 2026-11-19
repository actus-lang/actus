use std::collections::BTreeMap;

use llvm_sys::LLVMTypeKind::LLVMPointerTypeKind;
use llvm_sys::core::{
    LLVMBuildAlloca, LLVMBuildMemMove, LLVMConstInt, LLVMCreateBuilderInContext,
    LLVMDisposeBuilder, LLVMGetBasicBlockParent, LLVMGetEntryBasicBlock, LLVMGetFirstInstruction,
    LLVMGetInsertBlock, LLVMGetTypeContext, LLVMGetTypeKind, LLVMInt64TypeInContext,
    LLVMPositionBuilderAtEnd, LLVMPositionBuilderBefore, LLVMTypeOf,
};
use llvm_sys::prelude::{LLVMBuilderRef, LLVMTypeRef, LLVMValueRef};

use super::super::{BackendEmitError, BackendValueId};

/// A stack slot for one value of `ty`, allocated in the entry block of the
/// function `builder` is emitting (ADR-0114). Every slot is static: a loop
/// reuses its slot instead of growing the stack on each iteration, and
/// `mem2reg` and SROA can promote it. This is the only place the LLVM
/// backend calls `LLVMBuildAlloca`.
pub(super) fn entry_slot(
    builder: LLVMBuilderRef,
    ty: LLVMTypeRef,
    name: &std::ffi::CStr,
) -> Result<LLVMValueRef, BackendEmitError> {
    let block = unsafe { LLVMGetInsertBlock(builder) };
    if block.is_null() {
        return Err(BackendEmitError("LLVM stack slot requested outside a block".to_owned()));
    }
    let entry = unsafe { LLVMGetEntryBasicBlock(LLVMGetBasicBlockParent(block)) };
    let entry_builder = unsafe { LLVMCreateBuilderInContext(LLVMGetTypeContext(ty)) };
    let first = unsafe { LLVMGetFirstInstruction(entry) };
    unsafe {
        if first.is_null() {
            LLVMPositionBuilderAtEnd(entry_builder, entry);
        } else {
            LLVMPositionBuilderBefore(entry_builder, first);
        }
    }
    let slot = unsafe { LLVMBuildAlloca(entry_builder, ty, name.as_ptr()) };
    unsafe { LLVMDisposeBuilder(entry_builder) };
    (!slot.is_null())
        .then_some(slot)
        .ok_or_else(|| BackendEmitError("LLVM failed to allocate a stack slot".to_owned()))
}

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
