use std::collections::BTreeMap;

use super::super::{
    BackendBlockId, BackendEmitError, BackendFunction, BackendInstruction, BackendValueId,
    BackendValueType,
};
use super::branches::{block_value, emit_branch, emit_jump};
use super::calls::emit_call;
use super::cleanup::emit_cleanup;
use super::declarations::FunctionDeclaration;
use super::enums::{emit_enum_construct, emit_enum_discriminant, emit_enum_payload};
use super::failures::emit_failure;
use super::guards::{
    emit_trap_if_array_index_out_of_bounds, emit_trap_if_shift_count_out_of_range,
    emit_trap_if_signed_division_overflow, emit_trap_if_zero,
};
use super::integer_cast::emit_integer_cast;
use super::integer_ops::{emit_integer_binary, emit_integer_compare, emit_integer_unary};
use super::memory::emit_memory_copy;
use super::returns::emit_return;
use llvm_sys::core::{
    LLVMBuildRetVoid, LLVMBuildUnreachable, LLVMConstInt, LLVMGetParam, LLVMPositionBuilderAtEnd,
};
use llvm_sys::prelude::{LLVMBasicBlockRef, LLVMBuilderRef, LLVMValueRef};

pub(super) type BlockParameters = BTreeMap<BackendBlockId, Vec<(BackendValueId, LLVMValueRef)>>;

struct InstructionContext<'a> {
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    pointer_width: u8,
    function: &'a BackendFunction,
    declaration: FunctionDeclaration,
    declarations: &'a BTreeMap<String, FunctionDeclaration>,
    blocks: &'a BTreeMap<BackendBlockId, LLVMBasicBlockRef>,
    block_parameters: &'a BlockParameters,
    values: &'a mut BTreeMap<BackendValueId, LLVMValueRef>,
}

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
    for block in &function.blocks {
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
            if matches!(function.return_value.ty, BackendValueType::Unit) {
                unsafe { LLVMBuildRetVoid(builder) };
            } else {
                return Err(BackendEmitError(
                    "LLVM function block has no supported return".to_owned(),
                ));
            }
        }
    }
    Ok(())
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

fn emit_instruction(
    context: &mut InstructionContext<'_>,
    instruction: &BackendInstruction,
) -> Result<bool, BackendEmitError> {
    if let Some(result) = emit_scalar_instruction(
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
    )? {
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
    blocks: &BTreeMap<BackendBlockId, LLVMBasicBlockRef>,
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

fn emit_scalar_instruction(
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    function: LLVMValueRef,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Option<Result<bool, BackendEmitError>> {
    let result = match instruction {
        BackendInstruction::Constant { result, ty, value, .. } => {
            emit_constant(context, *result, *ty, *value, values)
        }
        BackendInstruction::IntegerBinary { .. } => {
            emit_integer_binary(builder, instruction, values)
        }
        BackendInstruction::IntegerCast { .. } => {
            emit_integer_cast(context, builder, function, instruction, values)
        }
        BackendInstruction::TrapIfZero { .. }
        | BackendInstruction::TrapIfSignedDivisionOverflow { .. }
        | BackendInstruction::TrapIfShiftCountOutOfRange { .. }
        | BackendInstruction::TrapIfArrayIndexOutOfBounds { .. } => {
            emit_guard(context, builder, function, instruction, values)
        }
        BackendInstruction::IntegerCompare { .. } => {
            emit_integer_compare(builder, instruction, values)
        }
        BackendInstruction::IntegerUnary { .. } => emit_integer_unary(builder, instruction, values),
        BackendInstruction::EnumDiscriminant { .. } => {
            emit_enum_discriminant(context, builder, instruction, values)
        }
        BackendInstruction::EnumPayload { .. } => {
            emit_enum_payload(context, builder, instruction, values)
        }
        BackendInstruction::EnumPayloadBinding { .. } => {
            emit_enum_payload(context, builder, instruction, values)
        }
        BackendInstruction::EnumConstruct { .. } => {
            emit_enum_construct(context, builder, instruction, values)
        }
        _ => return None,
    };
    Some(result)
}

fn emit_guard(
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    function: LLVMValueRef,
    instruction: &BackendInstruction,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    match instruction {
        BackendInstruction::TrapIfZero { .. } => {
            emit_trap_if_zero(context, builder, function, instruction, values)
        }
        BackendInstruction::TrapIfSignedDivisionOverflow { .. } => {
            emit_trap_if_signed_division_overflow(context, builder, function, instruction, values)
        }
        BackendInstruction::TrapIfShiftCountOutOfRange { .. } => {
            emit_trap_if_shift_count_out_of_range(context, builder, function, instruction, values)
        }
        BackendInstruction::TrapIfArrayIndexOutOfBounds { .. } => {
            emit_trap_if_array_index_out_of_bounds(context, builder, function, instruction, values)
        }
        _ => Err(BackendEmitError("LLVM guard instruction is not supported".to_owned())),
    }
}

fn emit_constant(
    context: llvm_sys::prelude::LLVMContextRef,
    result: BackendValueId,
    ty: BackendValueType,
    value: u64,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let llvm_type = super::values::llvm_type(context, ty)?;
    let constant = unsafe { LLVMConstInt(llvm_type, value, 0) };
    if constant.is_null() {
        return Err(BackendEmitError("LLVM failed to create integer constant".to_owned()));
    }
    values.insert(result, constant);
    Ok(false)
}
