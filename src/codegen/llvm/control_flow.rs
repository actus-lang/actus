use std::collections::BTreeMap;

use super::super::{
    BackendBlockId, BackendEmitError, BackendFunction, BackendInstruction, BackendValueId,
    BackendValueType,
};
use super::branches::block_value;
use super::declarations::FunctionDeclaration;
use super::instruction::{InstructionContext, emit_instruction};
use llvm_sys::core::{
    LLVMBuildRetVoid, LLVMBuildUnreachable, LLVMGetParam, LLVMPositionBuilderAtEnd,
};
use llvm_sys::prelude::{LLVMBasicBlockRef, LLVMBuilderRef, LLVMValueRef};

pub(super) type BlockParameters = BTreeMap<BackendBlockId, Vec<(BackendValueId, LLVMValueRef)>>;

pub(super) fn emit_blocks(
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    pointer_width: u8,
    function: &BackendFunction,
    declaration: FunctionDeclaration,
    declarations: &BTreeMap<String, FunctionDeclaration>,
    blocks: &BTreeMap<BackendBlockId, LLVMBasicBlockRef>,
) -> Result<(), BackendEmitError> {
    let mut values = parameter_values(declaration);
    let block_parameters = super::module::block_parameters(context, builder, function, blocks)?;
    values.extend(block_parameters.values().flatten().copied());
    for block in ordered_blocks(function) {
        let llvm_block = block_value(blocks, block.id)?;
        unsafe { LLVMPositionBuilderAtEnd(builder, llvm_block) };
        let mut terminated = false;
        for instruction in &block.instructions {
            if terminated {
                return Err(BackendEmitError(
                    "LLVM block has instructions after a terminator".to_owned(),
                ));
            }
            terminated = emit_instruction(
                &mut InstructionContext {
                    context,
                    builder,
                    pointer_width,
                    function,
                    declaration,
                    declarations,
                    blocks,
                    block_parameters: &block_parameters,
                    values: &mut values,
                },
                instruction,
            )?;
        }
        if !terminated {
            if block.id != function.entry
                && block.instructions.is_empty()
                && !function_has_predecessor(function, block.id)
            {
                unsafe { LLVMBuildUnreachable(builder) };
                continue;
            }
            if matches!(function.return_value.ty, BackendValueType::Unit) {
                unsafe { LLVMBuildRetVoid(builder) };
            } else {
                return Err(BackendEmitError(format!(
                    "LLVM function `{}` block {} has no supported return",
                    function.symbol.as_str(),
                    block.id.0
                )));
            }
        }
    }
    Ok(())
}

fn function_has_predecessor(function: &BackendFunction, target: BackendBlockId) -> bool {
    function.blocks.iter().any(|block| {
        block.instructions.iter().any(|instruction| match instruction {
            BackendInstruction::Branch { then_block, else_block, .. } => {
                *then_block == target || *else_block == target
            }
            BackendInstruction::Jump { target: jump_target, .. } => *jump_target == target,
            _ => false,
        })
    })
}

fn ordered_blocks(function: &BackendFunction) -> Vec<&super::super::BackendBlock> {
    let by_id = function.blocks.iter().map(|block| (block.id, block)).collect::<BTreeMap<_, _>>();
    let mut definitions = BTreeMap::new();
    for block in &function.blocks {
        for parameter in &block.parameters {
            definitions.insert(parameter.value, block.id);
        }
        for instruction in &block.instructions {
            if let Some(result) = instruction_result(instruction) {
                definitions.insert(result, block.id);
            }
        }
    }
    let mut dependencies = BTreeMap::<BackendBlockId, Vec<BackendBlockId>>::new();
    for block in &function.blocks {
        let mut required = block
            .instructions
            .iter()
            .flat_map(instruction_inputs)
            .filter_map(|value| definitions.get(&value).copied())
            .filter(|definition| *definition != block.id)
            .collect::<Vec<_>>();
        required.sort();
        required.dedup();
        dependencies.insert(block.id, required);
    }
    let mut ordered_ids = Vec::with_capacity(function.blocks.len());
    let mut remaining = function.blocks.iter().map(|block| block.id).collect::<Vec<_>>();
    while !remaining.is_empty() {
        let candidate = remaining
            .iter()
            .copied()
            .find(|id| {
                dependencies
                    .get(id)
                    .is_none_or(|items| items.iter().all(|item| ordered_ids.contains(item)))
            })
            .unwrap_or(remaining[0]);
        remaining.retain(|id| *id != candidate);
        ordered_ids.push(candidate);
    }
    ordered_ids.into_iter().filter_map(|id| by_id.get(&id).copied()).collect()
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
        | BackendInstruction::ArenaConstruct { result, .. }
        | BackendInstruction::ArenaPlace { result, .. }
        | BackendInstruction::AggregateConstruct { result, .. }
        | BackendInstruction::AggregateCopy { result, .. }
        | BackendInstruction::AggregateFieldLoad { result, .. }
        | BackendInstruction::PackedFieldLoad { result, .. }
        | BackendInstruction::ArrayElementLoad { result, .. }
        | BackendInstruction::ScalarLoanLoad { result, .. }
        | BackendInstruction::ScalarLoanMaterialize { result, .. } => Some(*result),
        BackendInstruction::Call { result, .. } => *result,
        _ => None,
    }
}

fn instruction_inputs(instruction: &BackendInstruction) -> Vec<BackendValueId> {
    match instruction {
        BackendInstruction::IntegerBinary { left, right, .. }
        | BackendInstruction::IntegerCompare { left, right, .. }
        | BackendInstruction::FloatCompare { left, right, .. } => vec![*left, *right],
        BackendInstruction::ScalarSelect { condition, then_value, else_value, .. } => {
            vec![*condition, *then_value, *else_value]
        }
        BackendInstruction::IntegerCast { source, .. }
        | BackendInstruction::FloatCast { source, .. }
        | BackendInstruction::IntegerUnary { operand: source, .. }
        | BackendInstruction::TrapIfZero { value: source, .. }
        | BackendInstruction::TrapIfShiftCountOutOfRange { count: source, .. }
        | BackendInstruction::TrapIfArrayIndexOutOfBounds { index: source, .. }
        | BackendInstruction::EnumDiscriminant { subject: source, .. }
        | BackendInstruction::EnumPayload { subject: source, .. }
        | BackendInstruction::EnumPayloadBinding { subject: source, .. } => vec![*source],
        BackendInstruction::TrapIfSignedDivisionOverflow { left, right, .. } => vec![*left, *right],
        BackendInstruction::EnumConstruct { payload, .. } => payload.iter().copied().collect(),
        BackendInstruction::Call { arguments, .. } => arguments.clone(),
        BackendInstruction::Branch { condition, then_arguments, else_arguments, .. } => {
            std::iter::once(*condition)
                .chain(then_arguments.iter().map(|argument| argument.value))
                .chain(else_arguments.iter().map(|argument| argument.value))
                .collect()
        }
        BackendInstruction::Jump { arguments, .. } => {
            arguments.iter().map(|argument| argument.value).collect()
        }
        BackendInstruction::Return { value, .. } => value.iter().copied().collect(),
        BackendInstruction::MemoryCopy { destination, source, .. } => vec![*destination, *source],
        BackendInstruction::ScalarLoanLoad { address, .. } => vec![*address],
        BackendInstruction::ScalarLoanStore { address, value, .. } => vec![*address, *value],
        BackendInstruction::ScalarLoanMaterialize { value, .. } => vec![*value],
        BackendInstruction::ArenaPlace { arena, value, .. } => vec![*arena, *value],
        BackendInstruction::AggregateConstruct { fields, .. } => {
            fields.iter().map(|field| field.value).collect()
        }
        BackendInstruction::AggregateCopy { source, .. } => vec![*source],
        BackendInstruction::AggregateFieldLoad { base, .. }
        | BackendInstruction::PackedFieldLoad { base, .. }
        | BackendInstruction::ArrayElementLoad { base, .. } => vec![*base],
        BackendInstruction::AggregateFieldStore { base, value, .. }
        | BackendInstruction::PackedFieldStore { base, value, .. } => vec![*base, *value],
        BackendInstruction::ArrayElementStore { base, index, value, .. } => {
            vec![*base, *index, *value]
        }
        BackendInstruction::Cleanup { operand, .. } => {
            operand.iter().map(|item| item.value).collect()
        }
        BackendInstruction::Constant { .. }
        | BackendInstruction::FloatConstant { .. }
        | BackendInstruction::StringLiteral { .. }
        | BackendInstruction::ArenaConstruct { .. }
        | BackendInstruction::Fail { .. }
        | BackendInstruction::Trap { .. } => Vec::new(),
    }
}

fn parameter_values(declaration: FunctionDeclaration) -> BTreeMap<BackendValueId, LLVMValueRef> {
    (0..declaration.parameter_count)
        .map(|index| {
            let physical_index = index + usize::from(declaration.return_slot);
            let value = unsafe { LLVMGetParam(declaration.value, physical_index as u32) };
            (BackendValueId(index as u32), value)
        })
        .collect()
}
