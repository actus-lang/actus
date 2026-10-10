use super::super::model::LoweredCleanupInstruction;
use super::{BackendContractError, BackendLayout, BackendValueId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackendCleanupField {
    pub offset: u32,
    pub size: u32,
    pub alignment: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackendCleanupOperand {
    pub value: BackendValueId,
    pub layout: BackendLayout,
    pub field: Option<BackendCleanupField>,
}

pub(super) fn validate_cleanup_operand(
    action: &LoweredCleanupInstruction,
    operand: Option<BackendCleanupOperand>,
) -> Result<(), BackendContractError> {
    let requires_value = !matches!(action, LoweredCleanupInstruction::EndBorrow { .. });
    if requires_value != operand.is_some() {
        return Err(BackendContractError::InvalidCleanupOperand);
    }
    let Some(operand) = operand else { return Ok(()) };
    operand.layout.validate()?;
    let Some(field) = operand.field else { return Ok(()) };
    if field.size == 0
        || field.alignment == 0
        || !field.alignment.is_power_of_two()
        || field.offset % field.alignment != 0
        || field.size % field.alignment != 0
        || field.offset > operand.layout.size
        || field.size > operand.layout.size - field.offset
    {
        return Err(BackendContractError::InvalidCleanupOperand);
    }
    Ok(())
}
