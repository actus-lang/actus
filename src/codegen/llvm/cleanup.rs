use super::super::BackendEmitError;
use super::super::model::LoweredCleanupInstruction;

pub(super) fn emit_cleanup(action: &LoweredCleanupInstruction) -> Result<bool, BackendEmitError> {
    if matches!(action, LoweredCleanupInstruction::EndBorrow { .. }) {
        return Ok(false);
    }
    Err(BackendEmitError(format!(
        "LLVM cleanup action `{}` requires a runtime bridge lowering",
        cleanup_name(action)
    )))
}

fn cleanup_name(action: &LoweredCleanupInstruction) -> &'static str {
    match action {
        LoweredCleanupInstruction::EndBorrow { .. } => "end-borrow",
        LoweredCleanupInstruction::DropPayloadField { .. } => "drop-payload-field",
        LoweredCleanupInstruction::DropBinding { .. } => "drop-binding",
        LoweredCleanupInstruction::DropBindingFields { .. } => "drop-binding-fields",
        LoweredCleanupInstruction::ResetArena { .. } => "reset-arena",
    }
}
