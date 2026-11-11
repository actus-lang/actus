use std::collections::BTreeMap;

use super::super::{BackendEmitError, BackendInstruction, BackendValueId};
use llvm_sys::core::{LLVMBuildAlloca, LLVMBuildLoad2, LLVMBuildStore};
use llvm_sys::prelude::{LLVMBuilderRef, LLVMValueRef};

pub(super) fn emit_scalar_loan(
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    match instruction {
        BackendInstruction::ScalarLoanLoad { result, address, ty, .. } => {
            emit_scalar_loan_load(context, builder, *result, *address, *ty, values)
        }
        BackendInstruction::ScalarLoanStore { address, value, .. } => {
            emit_scalar_loan_store(builder, *address, *value, values)
        }
        BackendInstruction::ScalarLoanMaterialize { result, value, ty, .. } => {
            emit_scalar_loan_materialize(context, builder, *result, *value, *ty, values)
        }
        _ => Err(BackendEmitError("LLVM scalar loan instruction expected".to_owned())),
    }
}

fn emit_scalar_loan_load(
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    result: BackendValueId,
    address_id: BackendValueId,
    ty: super::super::BackendValueType,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let address = values.get(&address_id).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM scalar loan address {} is unavailable", address_id.0))
    })?;
    let value_type = super::values::llvm_type(context, ty)?;
    let value =
        unsafe { LLVMBuildLoad2(builder, value_type, address, c"scalar.loan.load".as_ptr()) };
    if value.is_null() {
        return Err(BackendEmitError("LLVM failed to load scalar loan".to_owned()));
    }
    values.insert(result, value);
    Ok(false)
}

fn emit_scalar_loan_store(
    builder: LLVMBuilderRef,
    address_id: BackendValueId,
    value_id: BackendValueId,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let address = values.get(&address_id).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM scalar loan address {} is unavailable", address_id.0))
    })?;
    let value = values.get(&value_id).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM scalar loan value {} is unavailable", value_id.0))
    })?;
    if unsafe { LLVMBuildStore(builder, value, address) }.is_null() {
        return Err(BackendEmitError("LLVM failed to store scalar loan".to_owned()));
    }
    Ok(false)
}

fn emit_scalar_loan_materialize(
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    result: BackendValueId,
    value_id: BackendValueId,
    ty: super::super::BackendValueType,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let value = values.get(&value_id).copied().ok_or_else(|| {
        BackendEmitError(format!(
            "LLVM scalar loan materialization value {} is unavailable",
            value_id.0
        ))
    })?;
    let value_type = super::values::llvm_type(context, ty)?;
    let name = std::ffi::CString::new(format!("scalar.loan.slot{}", result.0))
        .map_err(|_| BackendEmitError("LLVM scalar loan slot name contains NUL".to_owned()))?;
    let address = unsafe { LLVMBuildAlloca(builder, value_type, name.as_ptr()) };
    if address.is_null() {
        return Err(BackendEmitError("LLVM failed to allocate scalar loan slot".to_owned()));
    }
    if unsafe { LLVMBuildStore(builder, value, address) }.is_null() {
        return Err(BackendEmitError("LLVM failed to store scalar loan value".to_owned()));
    }
    values.insert(result, address);
    Ok(false)
}
