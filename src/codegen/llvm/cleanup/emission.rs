use super::super::super::model::LoweredCleanupInstruction;
use super::super::super::{BackendCleanupOperand, BackendEmitError, BackendValueId};
use super::super::declarations::FunctionDeclaration;
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
