use std::collections::BTreeMap;

use super::super::{BackendEmitError, BackendInstruction, BackendValueId, BackendValueType};
use super::enums::{emit_enum_construct, emit_enum_discriminant, emit_enum_payload};
use super::float::{emit_float_cast, emit_float_compare, emit_float_constant};
use super::guards::{
    emit_trap_if_array_index_out_of_bounds, emit_trap_if_shift_count_out_of_range,
    emit_trap_if_signed_division_overflow, emit_trap_if_zero,
};
use super::integer::{
    emit_integer_binary, emit_integer_cast, emit_integer_compare, emit_integer_unary,
};
use super::literals::emit_string_literal;
use super::scalar_loan::emit_scalar_loan;
use llvm_sys::core::LLVMConstInt;
use llvm_sys::prelude::{LLVMBuilderRef, LLVMValueRef};

pub(super) fn emit_scalar_instruction(
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
        BackendInstruction::FloatConstant { .. } => {
            emit_float_constant(context, instruction, values)
        }
        BackendInstruction::StringLiteral { result, value, .. } => {
            emit_string_literal(builder, *result, value, values)
        }
        BackendInstruction::IntegerBinary { .. } => {
            emit_integer_binary(builder, instruction, values)
        }
        BackendInstruction::IntegerCast { .. } => {
            emit_integer_cast(context, builder, function, instruction, values)
        }
        BackendInstruction::FloatCast { .. } => {
            emit_float_cast(context, builder, instruction, values)
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
        BackendInstruction::FloatCompare { .. } => emit_float_compare(builder, instruction, values),
        BackendInstruction::IntegerUnary { .. } => emit_integer_unary(builder, instruction, values),
        BackendInstruction::EnumDiscriminant { .. } => {
            emit_enum_discriminant(context, builder, instruction, values)
        }
        BackendInstruction::EnumPayload { .. } | BackendInstruction::EnumPayloadBinding { .. } => {
            emit_enum_payload(context, builder, instruction, values)
        }
        BackendInstruction::EnumConstruct { .. } => {
            emit_enum_construct(context, builder, instruction, values)
        }
        BackendInstruction::ScalarLoanLoad { .. }
        | BackendInstruction::ScalarLoanStore { .. }
        | BackendInstruction::ScalarLoanMaterialize { .. } => {
            emit_scalar_loan(context, builder, instruction, values)
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
