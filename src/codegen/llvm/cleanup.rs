mod bridge;

use super::super::model::LoweredCleanupInstruction;
use super::super::{BackendCleanupOperand, BackendEmitError, BackendValueId};
use super::declarations::FunctionDeclaration;
use llvm_sys::prelude::{LLVMBuilderRef, LLVMContextRef, LLVMValueRef};
use std::collections::BTreeMap;

pub(super) fn emit_cleanup(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
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
    bridge::emit_runtime_bridge(context, builder, operand, declarations, values)
}
