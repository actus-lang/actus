// Dispatch receives the complete native lowering context explicitly.
#![allow(clippy::too_many_arguments)]

use std::collections::BTreeMap;

use cranelift_codegen::ir::{Block, InstBuilder, TrapCode};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::Module;
use cranelift_module::{DataDescription, Linkage};
use cranelift_object::ObjectModule;

use super::super::aggregate::{Declaration, lower_call};
use super::super::native::BackendEmitError;
use super::{aggregate, arena, cleanup, enums, returns};
use crate::codegen::{BackendFunction, BackendInstruction, BackendValueId};

type ValueMap = BTreeMap<BackendValueId, cranelift_codegen::ir::Value>;
type AggregateValueMap = BTreeMap<BackendValueId, Vec<cranelift_codegen::ir::Value>>;

pub(super) fn lower_instruction(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    declarations: &BTreeMap<String, Declaration>,
    blocks: &BTreeMap<crate::codegen::BackendBlockId, Block>,
    return_slot: Option<cranelift_codegen::ir::Value>,
    values: &mut ValueMap,
    aggregate_values: &mut AggregateValueMap,
    function: &BackendFunction,
) -> Result<(), BackendEmitError> {
    match instruction {
        BackendInstruction::Constant { .. }
        | BackendInstruction::StringLiteral { .. }
        | BackendInstruction::IntegerBinary { .. }
        | BackendInstruction::IntegerCast { .. }
        | BackendInstruction::TrapIfZero { .. }
        | BackendInstruction::TrapIfSignedDivisionOverflow { .. }
        | BackendInstruction::TrapIfShiftCountOutOfRange { .. }
        | BackendInstruction::TrapIfArrayIndexOutOfBounds { .. }
        | BackendInstruction::IntegerCompare { .. }
        | BackendInstruction::IntegerUnary { .. }
        | BackendInstruction::MemoryCopy { .. }
        | BackendInstruction::ScalarLoanLoad { .. }
        | BackendInstruction::ScalarLoanStore { .. }
        | BackendInstruction::ScalarLoanMaterialize { .. } => {
            lower_scalar(module, builder, instruction, values)
        }
        BackendInstruction::ArenaConstruct { .. } | BackendInstruction::ArenaPlace { .. } => {
            arena::lower(module, builder, instruction, values)
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
        BackendInstruction::Branch { .. } | BackendInstruction::Jump { .. } => {
            cf(builder, instruction, blocks, values)
        }
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
        _ if is_aggregate_instruction(instruction) => aggregate::lower_aggregate_instruction(
            module,
            builder,
            instruction,
            declarations,
            values,
        ),
        _ => unreachable!("Cranelift instruction dispatch is exhaustive"),
    }
}

fn is_aggregate_instruction(instruction: &BackendInstruction) -> bool {
    matches!(
        instruction,
        BackendInstruction::AggregateConstruct { .. }
            | BackendInstruction::AggregateFieldLoad { .. }
            | BackendInstruction::AggregateFieldStore { .. }
            | BackendInstruction::PackedFieldLoad { .. }
            | BackendInstruction::PackedFieldStore { .. }
            | BackendInstruction::ArrayElementLoad { .. }
            | BackendInstruction::ArrayElementStore { .. }
    )
}

fn cf(
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    blocks: &BTreeMap<crate::codegen::BackendBlockId, Block>,
    values: &mut ValueMap,
) -> Result<(), BackendEmitError> {
    match instruction {
        BackendInstruction::Branch { .. } => {
            super::lower_branch(builder, instruction, blocks, values)
        }
        BackendInstruction::Jump { .. } => super::lower_jump(builder, instruction, blocks, values),
        _ => Err(BackendEmitError("Cranelift control-flow instruction expected".to_owned())),
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
        BackendInstruction::StringLiteral { result, value, .. } => {
            lower_string_literal(module, builder, *result, value, values)
        }
        BackendInstruction::IntegerBinary { .. } => {
            super::lower_integer_binary(builder, instruction, values)
        }
        BackendInstruction::IntegerCast { .. } => {
            super::lower_integer_cast(builder, instruction, values)
        }
        BackendInstruction::TrapIfZero { .. }
        | BackendInstruction::TrapIfSignedDivisionOverflow { .. }
        | BackendInstruction::TrapIfShiftCountOutOfRange { .. }
        | BackendInstruction::TrapIfArrayIndexOutOfBounds { .. } => {
            super::lower_guard(builder, instruction, values)
        }
        BackendInstruction::IntegerCompare { .. } => {
            super::lower_integer_compare(builder, instruction, values)
        }
        BackendInstruction::IntegerUnary { .. } => {
            super::lower_integer_unary(builder, instruction, values)
        }
        BackendInstruction::MemoryCopy { .. } => {
            super::lower_memory_copy(module, builder, instruction, values)
        }
        BackendInstruction::ScalarLoanLoad { .. }
        | BackendInstruction::ScalarLoanStore { .. }
        | BackendInstruction::ScalarLoanMaterialize { .. } => {
            super::lower_scalar_loan(module, builder, instruction, values)
        }
        _ => Err(BackendEmitError("Cranelift scalar instruction expected".to_owned())),
    }
}

fn lower_string_literal(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    result: BackendValueId,
    value: &str,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    // Keep the neutral path's local data symbols in the established encoded
    // form used by the native emitter. The result id is deterministic within
    // a lowered module, so repeated builds retain stable object names.
    let symbol = format!("actus__data_string_5f{}", result.0);
    let data_id = module
        .declare_data(&symbol, Linkage::Local, false, false)
        .map_err(|error| BackendEmitError(error.to_string()))?;
    let mut description = DataDescription::new();
    description.define(format!("{value}\0").into_bytes().into_boxed_slice());
    module
        .define_data(data_id, &description)
        .map_err(|error| BackendEmitError(error.to_string()))?;
    let global = module.declare_data_in_func(data_id, builder.func);
    let pointer = builder.ins().symbol_value(module.isa().pointer_type(), global);
    values.insert(result, pointer);
    Ok(())
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
        BackendInstruction::Return { .. } => returns::lower_return(
            module,
            builder,
            instruction,
            function,
            return_slot,
            values,
            aggregate_values,
        ),
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
