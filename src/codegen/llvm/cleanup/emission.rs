use super::super::super::model::LoweredCleanupInstruction;
use super::super::super::{BackendCleanupOperand, BackendEmitError, BackendValueId};
use super::super::declarations::FunctionDeclaration;
use llvm_sys::core::{
    LLVMAppendBasicBlockInContext, LLVMBuildBr, LLVMBuildCondBr, LLVMGetBasicBlockParent,
    LLVMGetInsertBlock, LLVMPositionBuilderAtEnd,
};
use llvm_sys::prelude::{LLVMBuilderRef, LLVMContextRef, LLVMValueRef};
use std::collections::BTreeMap;

pub(in crate::codegen::llvm) fn emit_cleanup(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    pointer_width: u8,
    action: &LoweredCleanupInstruction,
    operand: Option<BackendCleanupOperand>,
    declarations: &BTreeMap<String, FunctionDeclaration>,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    if matches!(action, LoweredCleanupInstruction::EndBorrow { .. }) {
        return Ok(false);
    }
    let operand =
        operand.ok_or_else(|| BackendEmitError("LLVM cleanup operand is missing".to_owned()))?;
    super::bridge::emit_runtime_bridge(
        context,
        builder,
        pointer_width,
        operand,
        declarations,
        values,
    )
}

/// Emits a cleanup that runs only when its boolean condition holds, the
/// release of an enum payload that only the active variant owns (ADR-0125).
/// The cleanup gets a block of its own and the instructions that follow
/// continue in a new block that both paths reach.
pub(in crate::codegen::llvm) fn emit_conditional_cleanup(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    pointer_width: u8,
    (condition, action, operand): (
        BackendValueId,
        &LoweredCleanupInstruction,
        BackendCleanupOperand,
    ),
    declarations: &BTreeMap<String, FunctionDeclaration>,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let condition = values.get(&condition).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM cleanup condition {} is unavailable", condition.0))
    })?;
    let parent = unsafe { LLVMGetBasicBlockParent(LLVMGetInsertBlock(builder)) };
    let release =
        unsafe { LLVMAppendBasicBlockInContext(context, parent, c"cleanup.when".as_ptr().cast()) };
    let next =
        unsafe { LLVMAppendBasicBlockInContext(context, parent, c"cleanup.next".as_ptr().cast()) };
    if unsafe { LLVMBuildCondBr(builder, condition, release, next) }.is_null() {
        return Err(BackendEmitError("LLVM failed to branch around a cleanup".to_owned()));
    }
    unsafe { LLVMPositionBuilderAtEnd(builder, release) };
    emit_cleanup(context, builder, pointer_width, action, Some(operand), declarations, values)?;
    if unsafe { LLVMBuildBr(builder, next) }.is_null() {
        return Err(BackendEmitError("LLVM failed to join after a cleanup".to_owned()));
    }
    unsafe { LLVMPositionBuilderAtEnd(builder, next) };
    Ok(false)
}
