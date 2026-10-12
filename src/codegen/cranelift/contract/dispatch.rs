use std::collections::BTreeMap;

use cranelift_codegen::ir::{Block, InstBuilder, TrapCode};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::Module;
use cranelift_object::ObjectModule;

use super::super::aggregate::{Declaration, lower_call};
use super::super::native::BackendEmitError;
use super::{aggregate, cleanup, enums, returns};
use crate::codegen::{BackendFunction, BackendInstruction, BackendValueId};

pub(super) fn lower_instruction(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    declarations: &BTreeMap<String, Declaration>,
    blocks: &BTreeMap<crate::codegen::BackendBlockId, Block>,
    return_slot: Option<cranelift_codegen::ir::Value>,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
    aggregate_values: &mut BTreeMap<BackendValueId, Vec<cranelift_codegen::ir::Value>>,
    function: &BackendFunction,
) -> Result<(), BackendEmitError> {
    match instruction {
        BackendInstruction::Constant { .. }
        | BackendInstruction::IntegerBinary { .. }
        | BackendInstruction::IntegerCast { .. }
        | BackendInstruction::TrapIfZero { .. }
        | BackendInstruction::TrapIfSignedDivisionOverflow { .. }
        | BackendInstruction::TrapIfShiftCountOutOfRange { .. }
        | BackendInstruction::IntegerCompare { .. }
        | BackendInstruction::IntegerUnary { .. } => {
            lower_scalar(module, builder, instruction, values)
        }
        BackendInstruction::EnumDiscriminant { .. }
        | BackendInstruction::EnumPayload { .. }
        | BackendInstruction::EnumPayloadBinding { .. }
        | BackendInstruction::EnumConstruct { .. } => {
            enums::lower(module, builder, instruction, return_slot, values)
        }
        BackendInstruction::Call { .. } => {
            lower_call(module, builder, instruction, declarations, values, aggregate_values)
        }
        BackendInstruction::Branch { .. } => {
            super::lower_branch(builder, instruction, blocks, values)
        }
        BackendInstruction::Jump { .. } => super::lower_jump(builder, instruction, blocks, values),
        BackendInstruction::Return { .. }
        | BackendInstruction::Fail { .. }
        | BackendInstruction::Trap { .. }
        | BackendInstruction::Cleanup { .. } => lower_termination(
            module,
            builder,
            instruction,
            declarations,
            return_slot,
            values,
            aggregate_values,
            function,
        ),
        BackendInstruction::MemoryCopy { .. } => {
            super::lower_memory_copy(module, builder, instruction, values)
        }
        BackendInstruction::AggregateConstruct { .. }
        | BackendInstruction::AggregateFieldLoad { .. }
        | BackendInstruction::AggregateFieldStore { .. }
        | BackendInstruction::ArrayElementLoad { .. }
        | BackendInstruction::ArrayElementStore { .. } => {
            aggregate::lower_aggregate_instruction(module, builder, instruction, values)
        }
    }
}
fn lower_scalar(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    match instruction {
        BackendInstruction::Constant { .. } => {
            super::lower_constant(module, builder, instruction, values)
        }
        BackendInstruction::IntegerBinary { .. } => {
            super::lower_integer_binary(builder, instruction, values)
        }
        BackendInstruction::IntegerCast { .. } => {
            super::lower_integer_cast(builder, instruction, values)
        }
        BackendInstruction::TrapIfZero { .. }
        | BackendInstruction::TrapIfSignedDivisionOverflow { .. }
        | BackendInstruction::TrapIfShiftCountOutOfRange { .. } => {
            super::lower_guard(builder, instruction, values)
        }
        BackendInstruction::IntegerCompare { .. } => {
            super::lower_integer_compare(builder, instruction, values)
        }
        BackendInstruction::IntegerUnary { .. } => {
            super::lower_integer_unary(builder, instruction, values)
        }
        _ => Err(BackendEmitError("Cranelift scalar instruction expected".to_owned())),
    }
}

fn lower_termination(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    declarations: &BTreeMap<String, Declaration>,
    return_slot: Option<cranelift_codegen::ir::Value>,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
    aggregate_values: &mut BTreeMap<BackendValueId, Vec<cranelift_codegen::ir::Value>>,
    function: &BackendFunction,
) -> Result<(), BackendEmitError> {
    match instruction {
        BackendInstruction::Return { .. } => {
            returns::lower_return(builder, instruction, function, values, aggregate_values)
        }
        BackendInstruction::Fail { .. } => returns::lower_failure(
            module.isa().pointer_type(),
            builder,
            instruction,
            return_slot,
            values,
        ),
        BackendInstruction::Trap { .. } => {
            builder.ins().trap(TrapCode::unwrap_user(1));
            Ok(())
        }
        BackendInstruction::Cleanup { action, operand, .. } => {
            cleanup::lower_cleanup(module, builder, action, operand.clone(), declarations, values)
        }
        _ => Err(BackendEmitError("Cranelift termination instruction expected".to_owned())),
    }
}
