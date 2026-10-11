use std::collections::BTreeSet;

use super::super::{BackendAggregateField, BackendBlock, BackendBlockId};
use super::cleanup::validate_cleanup_operand;
use super::{BackendContractError, BackendInstruction, validate_value_type};

pub(super) fn validate_instructions(
    blocks: &[BackendBlock],
    block_ids: &BTreeSet<BackendBlockId>,
) -> Result<(), BackendContractError> {
    for block in blocks {
        for instruction in &block.instructions {
            validate_instruction(instruction, block_ids)?;
        }
    }
    Ok(())
}

fn validate_instruction(
    instruction: &BackendInstruction,
    block_ids: &BTreeSet<BackendBlockId>,
) -> Result<(), BackendContractError> {
    match instruction {
        BackendInstruction::Constant { ty, .. }
            if !matches!(
                ty,
                super::BackendValueType::Bool | super::BackendValueType::Integer { .. }
            ) =>
        {
            Err(BackendContractError::InvalidValueType)
        }
        BackendInstruction::Call { callee, .. } if callee.as_str().is_empty() => {
            Err(BackendContractError::InvalidCallSymbol)
        }
        BackendInstruction::Branch { then_block, else_block, .. }
            if !block_ids.contains(then_block) || !block_ids.contains(else_block) =>
        {
            Err(BackendContractError::InvalidBlockReference)
        }
        BackendInstruction::Jump { target, .. } if !block_ids.contains(target) => {
            Err(BackendContractError::InvalidBlockReference)
        }
        BackendInstruction::Fail { failure, .. } => super::super::abi::validate_failure(failure),
        BackendInstruction::EnumDiscriminant { layout, .. } => layout.validate(),
        BackendInstruction::EnumPayload { layout, payload_type, .. }
        | BackendInstruction::EnumPayloadBinding { layout, payload_type, .. } => {
            layout.validate()?;
            validate_value_type(*payload_type)
        }
        BackendInstruction::EnumConstruct { discriminant, layout, payload_type, .. } => {
            validate_enum_construct(*discriminant, *layout, *payload_type)
        }
        BackendInstruction::Cleanup { action, operand, .. } => {
            validate_cleanup_operand(action, operand.clone())
        }
        BackendInstruction::MemoryCopy { size, alignment, .. }
            if *size == 0
                || *alignment == 0
                || !alignment.is_power_of_two()
                || size % alignment != 0 =>
        {
            Err(BackendContractError::InvalidMemoryCopy)
        }
        BackendInstruction::AggregateConstruct { layout, fields, .. } => {
            validate_aggregate_construct(*layout, fields)
        }
        _ => Ok(()),
    }
}

fn validate_enum_construct(
    discriminant: u32,
    layout: super::BackendEnumLayout,
    payload_type: super::BackendValueType,
) -> Result<(), BackendContractError> {
    layout.validate()?;
    if layout.niche_pointer || layout.payload.is_none() {
        return Err(BackendContractError::InvalidEnumLayout(
            "enum constructor requires an inline payload",
        ));
    }
    if u64::from(discriminant) >= (1u64 << (u32::from(layout.discriminant_size) * 8)) {
        return Err(BackendContractError::InvalidEnumLayout(
            "enum constructor discriminant does not fit its field",
        ));
    }
    validate_value_type(payload_type)
}

fn validate_aggregate_construct(
    layout: super::BackendLayout,
    fields: &[BackendAggregateField],
) -> Result<(), BackendContractError> {
    layout.validate()?;
    let mut previous_end = 0u32;
    for field in fields {
        field.layout.validate()?;
        if field.offset % field.layout.alignment != 0
            || field.offset > layout.size
            || field.layout.size > layout.size - field.offset
            || field.offset < previous_end
        {
            return Err(BackendContractError::InvalidAggregateInstruction);
        }
        validate_value_type(field.ty)?;
        previous_end = field.offset + field.layout.size;
    }
    Ok(())
}
