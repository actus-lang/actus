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
            validate_instruction(instruction, blocks, block_ids)?;
        }
    }
    Ok(())
}

fn validate_instruction(
    instruction: &BackendInstruction,
    blocks: &[BackendBlock],
    block_ids: &BTreeSet<BackendBlockId>,
) -> Result<(), BackendContractError> {
    if let Some(result) = validate_control_flow(instruction, blocks, block_ids) {
        return result;
    }
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
        BackendInstruction::AggregateConstruct { .. }
        | BackendInstruction::AggregateFieldLoad { .. }
        | BackendInstruction::AggregateFieldStore { .. }
        | BackendInstruction::ArrayElementLoad { .. } => {
            validate_aggregate_instruction(instruction)
        }
        _ => Ok(()),
    }
}

fn validate_control_flow(
    instruction: &BackendInstruction,
    blocks: &[BackendBlock],
    block_ids: &BTreeSet<BackendBlockId>,
) -> Option<Result<(), BackendContractError>> {
    match instruction {
        BackendInstruction::Branch {
            then_block,
            else_block,
            then_arguments,
            else_arguments,
            ..
        } => Some(validate_branch_edges(
            blocks,
            block_ids,
            *then_block,
            then_arguments,
            *else_block,
            else_arguments,
        )),
        BackendInstruction::Jump { target, arguments, .. } => {
            Some(validate_jump_edge(blocks, block_ids, *target, arguments))
        }
        _ => None,
    }
}

fn validate_branch_edges(
    blocks: &[BackendBlock],
    block_ids: &BTreeSet<BackendBlockId>,
    then_block: BackendBlockId,
    then_arguments: &[super::BackendEdgeArgument],
    else_block: BackendBlockId,
    else_arguments: &[super::BackendEdgeArgument],
) -> Result<(), BackendContractError> {
    if !block_ids.contains(&then_block) || !block_ids.contains(&else_block) {
        return Err(BackendContractError::InvalidBlockReference);
    }
    validate_edge(blocks, then_block, then_arguments)?;
    validate_edge(blocks, else_block, else_arguments)
}

fn validate_jump_edge(
    blocks: &[BackendBlock],
    block_ids: &BTreeSet<BackendBlockId>,
    target: BackendBlockId,
    arguments: &[super::BackendEdgeArgument],
) -> Result<(), BackendContractError> {
    if !block_ids.contains(&target) {
        return Err(BackendContractError::InvalidBlockReference);
    }
    validate_edge(blocks, target, arguments)
}

fn validate_edge(
    blocks: &[BackendBlock],
    target: super::BackendBlockId,
    arguments: &[super::BackendEdgeArgument],
) -> Result<(), BackendContractError> {
    let block = blocks
        .iter()
        .find(|block| block.id == target)
        .ok_or(BackendContractError::InvalidBlockReference)?;
    super::validate_edge_arguments(&block.parameters, arguments)
}

fn validate_aggregate_instruction(
    instruction: &BackendInstruction,
) -> Result<(), BackendContractError> {
    match instruction {
        BackendInstruction::AggregateConstruct { layout, fields, .. } => {
            validate_aggregate_construct(*layout, fields)
        }
        BackendInstruction::AggregateFieldLoad {
            aggregate_layout,
            offset,
            field_layout,
            ty,
            ..
        } => validate_aggregate_field_load(*aggregate_layout, *offset, *field_layout, *ty),
        BackendInstruction::AggregateFieldStore {
            aggregate_layout,
            offset,
            field_layout,
            ty,
            ..
        } => validate_aggregate_field_store(*aggregate_layout, *offset, *field_layout, *ty),
        BackendInstruction::ArrayElementLoad {
            aggregate_layout,
            element_layout,
            element_type,
            capacity,
            ..
        } => validate_array_element_load(
            *aggregate_layout,
            *element_layout,
            *element_type,
            *capacity,
        ),
        _ => Ok(()),
    }
}

fn validate_array_element_load(
    aggregate_layout: super::BackendLayout,
    element_layout: super::BackendLayout,
    element_type: super::BackendValueType,
    capacity: u32,
) -> Result<(), BackendContractError> {
    aggregate_layout.validate()?;
    element_layout.validate()?;
    if capacity == 0
        || u64::from(element_layout.size) * u64::from(capacity) != u64::from(aggregate_layout.size)
    {
        return Err(BackendContractError::InvalidLayout(
            "array element load does not match aggregate capacity",
        ));
    }
    validate_value_type(element_type)
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

fn validate_aggregate_field_load(
    aggregate_layout: super::BackendLayout,
    offset: u32,
    field_layout: super::BackendLayout,
    ty: super::BackendValueType,
) -> Result<(), BackendContractError> {
    aggregate_layout.validate()?;
    field_layout.validate()?;
    let unsupported_type = matches!(
        ty,
        super::BackendValueType::Unit
            | super::BackendValueType::Float { .. }
            | super::BackendValueType::Aggregate { .. }
            | super::BackendValueType::Region { .. }
            | super::BackendValueType::Enum { .. }
    );
    if unsupported_type
        || offset % field_layout.alignment != 0
        || offset > aggregate_layout.size
        || field_layout.size > aggregate_layout.size - offset
    {
        return Err(BackendContractError::InvalidAggregateFieldAccess);
    }
    Ok(())
}

fn validate_aggregate_field_store(
    aggregate_layout: super::BackendLayout,
    offset: u32,
    field_layout: super::BackendLayout,
    ty: super::BackendValueType,
) -> Result<(), BackendContractError> {
    aggregate_layout.validate()?;
    field_layout.validate()?;
    let unsupported_type = matches!(
        ty,
        super::BackendValueType::Unit
            | super::BackendValueType::Float { .. }
            | super::BackendValueType::Aggregate { .. }
            | super::BackendValueType::Region { .. }
            | super::BackendValueType::Enum { .. }
    );
    if unsupported_type
        || offset % field_layout.alignment != 0
        || offset > aggregate_layout.size
        || field_layout.size > aggregate_layout.size - offset
    {
        return Err(BackendContractError::InvalidAggregateFieldAccess);
    }
    Ok(())
}
