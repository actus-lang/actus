use std::collections::BTreeMap;

use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{Block, BlockArg, InstBuilder};
use cranelift_frontend::FunctionBuilder;

use super::super::native::BackendEmitError;
use crate::codegen::{BackendInstruction, BackendValueId};

pub(super) fn lower_branch(
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    blocks: &BTreeMap<crate::codegen::BackendBlockId, Block>,
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::Branch {
        condition,
        then_block,
        else_block,
        then_arguments,
        else_arguments,
        ..
    } = instruction
    else {
        return Err(BackendEmitError("Cranelift branch instruction expected".to_owned()));
    };
    let condition = values.get(condition).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift branch value {} is unavailable", condition.0))
    })?;
    let condition = builder.ins().icmp_imm_u(IntCC::NotEqual, condition, 0);
    let then_block = blocks
        .get(then_block)
        .copied()
        .ok_or_else(|| BackendEmitError("Cranelift branch true block is unavailable".to_owned()))?;
    let else_block = blocks.get(else_block).copied().ok_or_else(|| {
        BackendEmitError("Cranelift branch false block is unavailable".to_owned())
    })?;
    let then_arguments = edge_arguments(then_arguments, values)?;
    let else_arguments = edge_arguments(else_arguments, values)?;
    builder.ins().brif(condition, then_block, &then_arguments, else_block, &else_arguments);
    Ok(())
}

pub(super) fn lower_jump(
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    blocks: &BTreeMap<crate::codegen::BackendBlockId, Block>,
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::Jump { target, arguments, .. } = instruction else {
        return Err(BackendEmitError("Cranelift jump instruction expected".to_owned()));
    };
    let target = blocks
        .get(target)
        .copied()
        .ok_or_else(|| BackendEmitError("Cranelift jump target block is unavailable".to_owned()))?;
    let arguments = edge_arguments(arguments, values)?;
    builder.ins().jump(target, &arguments);
    Ok(())
}

fn edge_arguments(
    arguments: &[crate::codegen::BackendEdgeArgument],
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<Vec<BlockArg>, BackendEmitError> {
    arguments
        .iter()
        .map(|argument| {
            values.get(&argument.value).copied().map(BlockArg::Value).ok_or_else(|| {
                BackendEmitError(format!(
                    "Cranelift edge argument value {} is unavailable",
                    argument.value.0
                ))
            })
        })
        .collect()
}
