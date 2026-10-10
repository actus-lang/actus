use crate::ast::{BuiltinType, TypeName};

use super::super::model::LoweredCleanupInstruction;
use super::{BackendContractError, BackendLayout, BackendValueId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendCleanupBridge {
    BufferDrop,
    RegionDrop,
    EnumDrop { size: u32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendCleanupArgument {
    Value,
    Address,
    FieldAddress,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendCleanupResult {
    Ignored,
    Status,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackendCleanupBridgeDescriptor {
    pub bridge: BackendCleanupBridge,
    pub argument: BackendCleanupArgument,
    pub result: BackendCleanupResult,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackendCleanupField {
    pub offset: u32,
    pub size: u32,
    pub alignment: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendCleanupResolvedDescriptor {
    pub descriptor: BackendCleanupBridgeDescriptor,
    pub layout: BackendLayout,
    pub field: Option<BackendCleanupField>,
}

pub trait BackendCleanupProvider {
    fn resolve_binding(
        &self,
        type_name: Option<&TypeName>,
        builtin: Option<BuiltinType>,
    ) -> Result<Option<BackendCleanupResolvedDescriptor>, BackendContractError>;

    fn resolve_field(
        &self,
        root_type: &TypeName,
        path: &str,
    ) -> Result<Option<BackendCleanupResolvedDescriptor>, BackendContractError>;
}

impl BackendCleanupResolvedDescriptor {
    /// Materialize the validated bridge metadata as the operand consumed by a backend.
    pub fn operand(&self, value: BackendValueId) -> BackendCleanupOperand {
        BackendCleanupOperand {
            value,
            layout: self.layout,
            field: self.field,
            bridge: self.descriptor.bridge,
            argument: self.descriptor.argument,
            result: self.descriptor.result,
        }
    }

    pub fn validate(&self) -> Result<(), BackendContractError> {
        self.layout.validate()?;
        if matches!(self.descriptor.argument, BackendCleanupArgument::FieldAddress)
            != self.field.is_some()
        {
            return Err(BackendContractError::InvalidCleanupOperand);
        }
        let Some(field) = self.field else { return Ok(()) };
        if field.size == 0
            || field.alignment == 0
            || !field.alignment.is_power_of_two()
            || field.offset % field.alignment != 0
            || field.size % field.alignment != 0
            || field.offset > self.layout.size
            || field.size > self.layout.size - field.offset
        {
            return Err(BackendContractError::InvalidCleanupOperand);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackendCleanupOperand {
    pub value: BackendValueId,
    pub layout: BackendLayout,
    pub field: Option<BackendCleanupField>,
    pub bridge: BackendCleanupBridge,
    pub argument: BackendCleanupArgument,
    pub result: BackendCleanupResult,
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
    if matches!(operand.bridge, BackendCleanupBridge::EnumDrop { size: 0 }) {
        return Err(BackendContractError::InvalidCleanupOperand);
    }
    if matches!(operand.argument, BackendCleanupArgument::FieldAddress) != operand.field.is_some() {
        return Err(BackendContractError::InvalidCleanupOperand);
    }
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
