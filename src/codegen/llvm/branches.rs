use std::collections::BTreeMap;

use llvm_sys::core::{LLVMAddIncoming, LLVMBuildBr, LLVMBuildCondBr, LLVMGetInsertBlock};
use llvm_sys::prelude::{LLVMBasicBlockRef, LLVMBuilderRef, LLVMValueRef};

use super::super::{BackendBlockId, BackendEmitError, BackendInstruction, BackendValueId};
use super::control_flow::BlockParameters;

pub(super) fn emit_branch(
    builder: LLVMBuilderRef,
    instruction: &BackendInstruction,
    condition_id: BackendValueId,
    then_id: BackendBlockId,
    else_id: BackendBlockId,
    blocks: &BTreeMap<BackendBlockId, LLVMBasicBlockRef>,
    block_parameters: &BlockParameters,
    _current_block: BackendBlockId,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::Branch { then_arguments, else_arguments, .. } = instruction else {
        return Err(BackendEmitError("LLVM branch instruction expected".to_owned()));
    };
    let condition = values.get(&condition_id).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM branch value {} is unavailable", condition_id.0))
    })?;
    let then_block = block_value(blocks, then_id)?;
    let else_block = block_value(blocks, else_id)?;
    let source = unsafe { LLVMGetInsertBlock(builder) };
    if source.is_null() {
        return Err(BackendEmitError("LLVM branch has no insertion block".to_owned()));
    }
    add_incoming(block_parameters, then_id, then_arguments, source, values)?;
    add_incoming(block_parameters, else_id, else_arguments, source, values)?;
    unsafe { LLVMBuildCondBr(builder, condition, then_block, else_block) };
    Ok(())
}

pub(super) fn emit_jump(
    builder: LLVMBuilderRef,
    instruction: &BackendInstruction,
    target_id: BackendBlockId,
    blocks: &BTreeMap<BackendBlockId, LLVMBasicBlockRef>,
    block_parameters: &BlockParameters,
    _current_block: BackendBlockId,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::Jump { arguments, .. } = instruction else {
        return Err(BackendEmitError("LLVM jump instruction expected".to_owned()));
    };
    let target = block_value(blocks, target_id)?;
    let source = unsafe { LLVMGetInsertBlock(builder) };
    if source.is_null() {
        return Err(BackendEmitError("LLVM jump has no insertion block".to_owned()));
    }
    add_incoming(block_parameters, target_id, arguments, source, values)?;
    unsafe { LLVMBuildBr(builder, target) };
    Ok(())
}

fn add_incoming(
    block_parameters: &BlockParameters,
    target: BackendBlockId,
    arguments: &[super::super::BackendEdgeArgument],
    source: LLVMBasicBlockRef,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<(), BackendEmitError> {
    let parameters = block_parameters.get(&target).map(Vec::as_slice).unwrap_or(&[]);
    if parameters.len() != arguments.len() {
        return Err(BackendEmitError(format!(
            "LLVM edge argument count does not match block {} parameters",
            target.0
        )));
    }
    for ((_, phi), argument) in parameters.iter().zip(arguments) {
        let value = values.get(&argument.value).copied().ok_or_else(|| {
            BackendEmitError(format!(
                "LLVM edge argument value {} is unavailable",
                argument.value.0
            ))
        })?;
        let mut incoming_value = value;
        let mut incoming_block = source;
        unsafe { LLVMAddIncoming(*phi, &mut incoming_value, &mut incoming_block, 1) };
    }
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
