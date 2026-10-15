use std::collections::BTreeMap;

use super::super::{BackendEmitError, BackendFunction, BackendInstruction, BackendValueId};
use super::branches::{emit_branch, emit_jump};
use super::calls::emit_call;
use super::cleanup::emit_cleanup;
use super::control_flow::BlockParameters;
use super::declarations::FunctionDeclaration;
use super::failures::emit_failure;
use super::memory::emit_memory_copy;
use super::returns::emit_return;
use llvm_sys::core::LLVMBuildUnreachable;
use llvm_sys::prelude::{LLVMBuilderRef, LLVMValueRef};

pub(super) struct InstructionContext<'a> {
    pub(super) context: llvm_sys::prelude::LLVMContextRef,
    pub(super) builder: LLVMBuilderRef,
    pub(super) pointer_width: u8,
    pub(super) function: &'a BackendFunction,
    pub(super) declaration: FunctionDeclaration,
    pub(super) declarations: &'a BTreeMap<String, FunctionDeclaration>,
    pub(super) blocks:
        &'a BTreeMap<super::super::BackendBlockId, llvm_sys::prelude::LLVMBasicBlockRef>,
    pub(super) block_parameters: &'a BlockParameters,
    pub(super) values: &'a mut BTreeMap<BackendValueId, LLVMValueRef>,
}

pub(super) fn emit_instruction(
    context: &mut InstructionContext<'_>,
    instruction: &BackendInstruction,
) -> Result<bool, BackendEmitError> {
    if let Some(result) = super::scalar::emit_scalar_instruction(
        context.context,
        context.builder,
        context.declaration.value,
        instruction,
        context.values,
    ) {
        return result;
    }
    emit_non_scalar_instruction(context, instruction)
}

fn emit_non_scalar_instruction(
    context: &mut InstructionContext<'_>,
    instruction: &BackendInstruction,
) -> Result<bool, BackendEmitError> {
    if let Some(result) = emit_control_transfer(
        context.builder,
        instruction,
        context.blocks,
        context.block_parameters,
        context.values,
    )
    .map_err(|error| {
        BackendEmitError(format!("LLVM function `{}`: {error}", context.function.symbol.as_str()))
    })? {
        return Ok(result);
    }
    emit_non_control_instruction(context, instruction)
}

fn emit_non_control_instruction(
    context: &mut InstructionContext<'_>,
    instruction: &BackendInstruction,
) -> Result<bool, BackendEmitError> {
    if is_aggregate_instruction(instruction) {
        return super::aggregates::emit_aggregate_instruction(
            context.context,
            context.builder,
            context.pointer_width,
            instruction,
            context.declarations,
            context.values,
        );
    }
    match instruction {
        BackendInstruction::Return { .. }
        | BackendInstruction::Call { .. }
        | BackendInstruction::Trap { .. }
        | BackendInstruction::Fail { .. } => emit_terminal_instruction(context, instruction),
        BackendInstruction::Cleanup { .. }
        | BackendInstruction::MemoryCopy { .. }
        | BackendInstruction::AggregateFieldStore { .. } => {
            emit_runtime_instruction(context, instruction)
        }
        _ => unreachable!("scalar instruction was not dispatched"),
    }
}

fn emit_terminal_instruction(
    context: &mut InstructionContext<'_>,
    instruction: &BackendInstruction,
) -> Result<bool, BackendEmitError> {
    match instruction {
        BackendInstruction::Return { value, .. } => {
            emit_return(
                context.context,
                context.builder,
                context.function,
                context.declaration,
                *value,
                context.values,
            )?;
            Ok(true)
        }
        BackendInstruction::Call { callee, arguments, result, .. } => {
            emit_call(
                context.context,
                context.builder,
                callee.as_str(),
                arguments,
                *result,
                context.declarations,
                context.values,
            )?;
            Ok(false)
        }
        BackendInstruction::Trap { .. } => {
            unsafe { LLVMBuildUnreachable(context.builder) };
            Ok(true)
        }
        BackendInstruction::Fail { failure, .. } => {
            emit_failure(
                context.context,
                context.builder,
                context.function,
                context.declaration,
                failure,
                context.values,
            )?;
            Ok(true)
        }
        _ => Err(BackendEmitError("LLVM terminal instruction expected".to_owned())),
    }
}

fn emit_runtime_instruction(
    context: &mut InstructionContext<'_>,
    instruction: &BackendInstruction,
) -> Result<bool, BackendEmitError> {
    match instruction {
        BackendInstruction::Cleanup { action, operand, .. } => emit_cleanup(
            context.context,
            context.builder,
            context.pointer_width,
            action,
            operand.clone(),
            context.declarations,
            context.values,
        ),
        BackendInstruction::MemoryCopy { .. } => emit_memory_copy_instruction(
            context.context,
            context.builder,
            instruction,
            context.values,
        ),
        BackendInstruction::AggregateFieldStore { .. } => {
            super::aggregates::emit_aggregate_field_store(
                context.context,
                context.builder,
                instruction,
                context.values,
            )
        }
        _ => Err(BackendEmitError("LLVM runtime instruction expected".to_owned())),
    }
}

fn is_aggregate_instruction(instruction: &BackendInstruction) -> bool {
    matches!(
        instruction,
        BackendInstruction::AggregateConstruct { .. }
            | BackendInstruction::AggregateFieldLoad { .. }
            | BackendInstruction::PackedFieldLoad { .. }
            | BackendInstruction::PackedFieldStore { .. }
            | BackendInstruction::ArenaConstruct { .. }
            | BackendInstruction::ArenaPlace { .. }
            | BackendInstruction::ArrayElementLoad { .. }
            | BackendInstruction::ArrayElementStore { .. }
    )
}

fn emit_control_transfer(
    builder: LLVMBuilderRef,
    instruction: &BackendInstruction,
    blocks: &BTreeMap<super::super::BackendBlockId, llvm_sys::prelude::LLVMBasicBlockRef>,
    block_parameters: &BlockParameters,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<Option<bool>, BackendEmitError> {
    match instruction {
        BackendInstruction::Branch { .. } => {
            emit_branch(builder, instruction, blocks, block_parameters, values)?;
            Ok(Some(true))
        }
        BackendInstruction::Jump { .. } => {
            emit_jump(builder, instruction, blocks, block_parameters, values)?;
            Ok(Some(true))
        }
        _ => Ok(None),
    }
}

fn emit_memory_copy_instruction(
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    instruction: &BackendInstruction,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let BackendInstruction::MemoryCopy { destination, source, size, alignment, .. } = instruction
    else {
        return Err(BackendEmitError("LLVM memory-copy instruction expected".to_owned()));
    };
    emit_memory_copy(context, builder, *destination, *source, *size, *alignment, values)
}
