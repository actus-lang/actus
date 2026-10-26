use std::collections::{BTreeMap, BTreeSet};

use super::super::{BackendBlock, BackendBlockId, BackendParameter};
use super::{BackendContractError, BackendInstruction, BackendValueId};

#[derive(Clone, Copy, Debug)]
struct ValueDefinition {
    block: BackendBlockId,
    instruction: usize,
}

pub(super) fn validate_cleanup_value_dominance(
    entry: BackendBlockId,
    parameters: &[BackendParameter],
    blocks: &[BackendBlock],
) -> Result<(), BackendContractError> {
    let definitions = collect_value_definitions(entry, parameters, blocks);
    let dominators = calculate_dominators(entry, blocks);
    for block in blocks {
        for (instruction, operation) in block.instructions.iter().enumerate() {
            let Some(BackendInstruction::Cleanup { operand: Some(operand), .. }) = Some(operation)
            else {
                continue;
            };
            let Some(definition) = definitions.get(&operand.value) else {
                return Err(BackendContractError::InvalidCleanupOperand);
            };
            let valid = definition.block == block.id
                && (definition.instruction == usize::MAX || definition.instruction < instruction)
                || definition.block != block.id
                    && dominators
                        .get(&block.id)
                        .is_some_and(|blocks| blocks.contains(&definition.block));
            if !valid {
                return Err(BackendContractError::InvalidCleanupOperand);
            }
        }
    }
    Ok(())
}

fn collect_value_definitions(
    entry: BackendBlockId,
    parameters: &[BackendParameter],
    blocks: &[BackendBlock],
) -> BTreeMap<BackendValueId, ValueDefinition> {
    let mut definitions: BTreeMap<BackendValueId, ValueDefinition> = parameters
        .iter()
        .enumerate()
        .map(|(index, _)| {
            (
                BackendValueId(index as u32),
                ValueDefinition { block: entry, instruction: usize::MAX },
            )
        })
        .collect();
    for block in blocks {
        for parameter in &block.parameters {
            definitions.insert(
                parameter.value,
                ValueDefinition { block: block.id, instruction: usize::MAX },
            );
        }
        for (instruction, operation) in block.instructions.iter().enumerate() {
            if let Some(value) = instruction_result(operation) {
                definitions.insert(value, ValueDefinition { block: block.id, instruction });
            }
        }
    }
    definitions
}

fn instruction_result(instruction: &BackendInstruction) -> Option<BackendValueId> {
    match instruction {
        BackendInstruction::Constant { result, .. }
        | BackendInstruction::FloatConstant { result, .. }
        | BackendInstruction::StringLiteral { result, .. }
        | BackendInstruction::IntegerBinary { result, .. }
        | BackendInstruction::IntegerCast { result, .. }
        | BackendInstruction::FloatCast { result, .. }
        | BackendInstruction::IntegerCompare { result, .. }
        | BackendInstruction::ScalarSelect { result, .. }
        | BackendInstruction::FloatCompare { result, .. }
        | BackendInstruction::EnumDiscriminant { result, .. }
        | BackendInstruction::EnumPayload { result, .. }
        | BackendInstruction::EnumPayloadBinding { result, .. }
        | BackendInstruction::EnumConstruct { result, .. }
        | BackendInstruction::IntegerUnary { result, .. }
        | BackendInstruction::ScalarLoanLoad { result, .. }
        | BackendInstruction::ScalarLoanMaterialize { result, .. }
        | BackendInstruction::ArenaConstruct { result, .. }
        | BackendInstruction::ArenaPlace { result, .. }
        | BackendInstruction::AggregateConstruct { result, .. }
        | BackendInstruction::AggregateCopy { result, .. }
        | BackendInstruction::AggregateFieldLoad { result, .. }
        | BackendInstruction::PackedFieldLoad { result, .. }
        | BackendInstruction::ArrayElementLoad { result, .. } => Some(*result),
        BackendInstruction::Call { result, .. } => *result,
        _ => None,
    }
}

fn calculate_dominators(
    entry: BackendBlockId,
    blocks: &[BackendBlock],
) -> BTreeMap<BackendBlockId, BTreeSet<BackendBlockId>> {
    let ids = blocks.iter().map(|block| block.id).collect::<BTreeSet<_>>();
    let mut predecessors =
        blocks.iter().map(|block| (block.id, BTreeSet::new())).collect::<BTreeMap<_, _>>();
    for block in blocks {
        for target in block_targets(block) {
            if ids.contains(&target) {
                predecessors.entry(target).or_default().insert(block.id);
            }
        }
    }
    let mut dominators = ids
        .iter()
        .map(|id| (*id, if *id == entry { BTreeSet::from([*id]) } else { ids.clone() }))
        .collect::<BTreeMap<_, _>>();
    loop {
        let mut changed = false;
        for id in ids.iter().filter(|id| **id != entry) {
            let Some(preds) = predecessors.get(id) else { continue };
            let Some(first) = preds.iter().next() else { continue };
            let mut next = dominators[first].clone();
            for predecessor in preds.iter().skip(1) {
                next = next.intersection(&dominators[predecessor]).copied().collect();
            }
            next.insert(*id);
            changed |= dominators[id] != next;
            dominators.insert(*id, next);
        }
        if !changed {
            return dominators;
        }
    }
}

fn block_targets(block: &BackendBlock) -> Vec<BackendBlockId> {
    block.instructions.last().map_or_else(Vec::new, |instruction| match instruction {
        BackendInstruction::Branch { then_block, else_block, .. } => vec![*then_block, *else_block],
        BackendInstruction::Jump { target, .. } => vec![*target],
        _ => Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::{BackendCleanupAction, BackendCleanupArgument, BackendCleanupBridge};
    use crate::lexer::SourceSpan;

    #[test]
    fn rejects_cleanup_value_defined_only_on_sibling_branch() {
        let span = SourceSpan { start: 0, end: 1 };
        let operand = super::super::BackendCleanupOperand {
            value: BackendValueId(1),
            layout: super::super::BackendLayout { size: 8, alignment: 8 },
            field: None,
            bridge: BackendCleanupBridge::BufferDrop,
            argument: BackendCleanupArgument::Value,
            result: super::super::BackendCleanupResult::Ignored,
            performance_symbol: None,
        };
        let blocks = vec![
            BackendBlock {
                id: BackendBlockId(0),
                parameters: Vec::new(),
                instructions: vec![BackendInstruction::Branch {
                    condition: BackendValueId(0),
                    then_block: BackendBlockId(1),
                    else_block: BackendBlockId(2),
                    then_arguments: Vec::new(),
                    else_arguments: Vec::new(),
                    span,
                }],
            },
            BackendBlock {
                id: BackendBlockId(1),
                parameters: Vec::new(),
                instructions: vec![BackendInstruction::Constant {
                    result: BackendValueId(1),
                    ty: super::super::BackendValueType::Bool,
                    value: 1,
                    span,
                }],
            },
            BackendBlock {
                id: BackendBlockId(2),
                parameters: Vec::new(),
                instructions: vec![BackendInstruction::Cleanup {
                    action: BackendCleanupAction::DropBinding {
                        binding_index: 0,
                        name: "buffer".to_owned(),
                    },
                    operand: Some(operand),
                    span,
                }],
            },
        ];
        assert_eq!(
            validate_cleanup_value_dominance(BackendBlockId(0), &[], &blocks),
            Err(BackendContractError::InvalidCleanupOperand)
        );
    }
}
