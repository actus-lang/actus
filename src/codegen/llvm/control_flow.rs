use std::collections::BTreeMap;

use llvm_sys::core::{
    LLVMBuildBr, LLVMBuildCondBr, LLVMBuildRetVoid, LLVMBuildUnreachable, LLVMConstInt,
    LLVMGetParam, LLVMPositionBuilderAtEnd,
};
use llvm_sys::prelude::{LLVMBasicBlockRef, LLVMBuilderRef, LLVMValueRef};

use super::super::model::LoweredCleanupInstruction;
use super::super::{
    BackendBlockId, BackendEmitError, BackendFunction, BackendInstruction, BackendValueId,
    BackendValueType,
};
use super::calls::emit_call;
use super::declarations::FunctionDeclaration;
use super::failures::emit_failure;
use super::guards::{emit_trap_if_shift_count_out_of_range, emit_trap_if_zero};
use super::integer_ops::{emit_integer_binary, emit_integer_compare, emit_integer_unary};
use super::memory::emit_memory_copy;
use super::returns::emit_return;

pub(super) fn emit_blocks(
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    function: &BackendFunction,
    declaration: FunctionDeclaration,
    declarations: &BTreeMap<String, FunctionDeclaration>,
    blocks: &BTreeMap<BackendBlockId, LLVMBasicBlockRef>,
) -> Result<(), BackendEmitError> {
    let mut values = parameter_values(declaration);
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
                context,
                builder,
                function,
                declaration,
                instruction,
                declarations,
                blocks,
                &mut values,
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
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    function: &BackendFunction,
    declaration: FunctionDeclaration,
    instruction: &BackendInstruction,
    declarations: &BTreeMap<String, FunctionDeclaration>,
    blocks: &BTreeMap<BackendBlockId, LLVMBasicBlockRef>,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    match instruction {
        BackendInstruction::Constant { result, ty, value, .. } => {
            emit_constant(context, *result, *ty, *value, values)
        }
        BackendInstruction::IntegerBinary { .. } => {
            emit_integer_binary(builder, instruction, values)
        }
        BackendInstruction::TrapIfZero { .. }
        | BackendInstruction::TrapIfShiftCountOutOfRange { .. } => {
            emit_guard(context, builder, declaration.value, instruction, values)
        }
        BackendInstruction::IntegerCompare { .. } => {
            emit_integer_compare(builder, instruction, values)
        }
        BackendInstruction::IntegerUnary { .. } => emit_integer_unary(builder, instruction, values),
        BackendInstruction::Return { value, .. } => {
            emit_return(builder, function, declaration, *value, values)?;
            Ok(true)
        }
        BackendInstruction::Call { callee, arguments, result, .. } => {
            emit_call(builder, callee.as_str(), arguments, *result, declarations, values)?;
            Ok(false)
        }
        BackendInstruction::Branch { condition, then_block, else_block, .. } => {
            emit_branch(builder, *condition, *then_block, *else_block, blocks, values)?;
            Ok(true)
        }
        BackendInstruction::Jump { target, .. } => {
            emit_jump(builder, *target, blocks)?;
            Ok(true)
        }
        BackendInstruction::Trap { .. } => {
            unsafe { LLVMBuildUnreachable(builder) };
            Ok(true)
        }
        BackendInstruction::Fail { failure, .. } => {
            emit_failure(context, builder, function, declaration, failure, values)?;
            Ok(true)
        }
        BackendInstruction::Cleanup { action, .. } => emit_cleanup(action),
        BackendInstruction::MemoryCopy { destination, source, size, alignment, .. } => {
            emit_memory_copy(context, builder, *destination, *source, *size, *alignment, values)
        }
    }
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
        BackendInstruction::TrapIfShiftCountOutOfRange { .. } => {
            emit_trap_if_shift_count_out_of_range(context, builder, function, instruction, values)
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

fn emit_cleanup(action: &LoweredCleanupInstruction) -> Result<bool, BackendEmitError> {
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

fn emit_branch(
    builder: LLVMBuilderRef,
    condition_id: BackendValueId,
    then_id: BackendBlockId,
    else_id: BackendBlockId,
    blocks: &BTreeMap<BackendBlockId, LLVMBasicBlockRef>,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<(), BackendEmitError> {
    let condition = values.get(&condition_id).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM branch value {} is unavailable", condition_id.0))
    })?;
    let then_block = block_value(blocks, then_id)?;
    let else_block = block_value(blocks, else_id)?;
    unsafe { LLVMBuildCondBr(builder, condition, then_block, else_block) };
    Ok(())
}

fn emit_jump(
    builder: LLVMBuilderRef,
    target: BackendBlockId,
    blocks: &BTreeMap<BackendBlockId, LLVMBasicBlockRef>,
) -> Result<(), BackendEmitError> {
    let target = block_value(blocks, target)?;
    unsafe { LLVMBuildBr(builder, target) };
    Ok(())
}

fn block_value(
    blocks: &BTreeMap<BackendBlockId, LLVMBasicBlockRef>,
    id: BackendBlockId,
) -> Result<LLVMBasicBlockRef, BackendEmitError> {
    blocks
        .get(&id)
        .copied()
        .ok_or_else(|| BackendEmitError(format!("LLVM block {} is unavailable", id.0)))
}
