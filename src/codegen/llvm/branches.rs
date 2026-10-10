use std::collections::BTreeMap;

use llvm_sys::core::{LLVMBuildBr, LLVMBuildCondBr};
use llvm_sys::prelude::{LLVMBasicBlockRef, LLVMBuilderRef, LLVMValueRef};

use super::super::{BackendBlockId, BackendEmitError, BackendValueId};

pub(super) fn emit_branch(
    builder: LLVMBuilderRef,
    condition_id: BackendValueId,
    then_id: BackendBlockId,
    else_id: BackendBlockId,
    blocks: &BTreeMap<BackendBlockId, LLVMBasicBlockRef>,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<(), BackendEmitError> {
    let condition = values.get(&condition_id).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM branch value {} is unavailable", condition_id.0))
    })?;
    let then_block = block_value(blocks, then_id)?;
    let else_block = block_value(blocks, else_id)?;
    unsafe { LLVMBuildCondBr(builder, condition, then_block, else_block) };
    Ok(())
}

pub(super) fn emit_jump(
    builder: LLVMBuilderRef,
    target: BackendBlockId,
    blocks: &BTreeMap<BackendBlockId, LLVMBasicBlockRef>,
) -> Result<(), BackendEmitError> {
    let target = block_value(blocks, target)?;
    unsafe { LLVMBuildBr(builder, target) };
    Ok(())
}

pub(super) fn block_value(
    blocks: &BTreeMap<BackendBlockId, LLVMBasicBlockRef>,
    id: BackendBlockId,
) -> Result<LLVMBasicBlockRef, BackendEmitError> {
    blocks
        .get(&id)
        .copied()
        .ok_or_else(|| BackendEmitError(format!("LLVM block {} is unavailable", id.0)))
}
