use std::collections::BTreeMap;

use llvm_sys::core::{LLVMBuildRet, LLVMBuildRetVoid};
use llvm_sys::prelude::{LLVMBuilderRef, LLVMValueRef};

use super::super::{BackendEmitError, BackendFunction, BackendValueId, BackendValueType};
use super::declarations::FunctionDeclaration;

pub(super) fn emit_return(
    builder: LLVMBuilderRef,
    function: &BackendFunction,
    declaration: FunctionDeclaration,
    value_id: Option<BackendValueId>,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<(), BackendEmitError> {
    match value_id {
        None if declaration.return_slot => {
            Err(BackendEmitError("LLVM return-slot function requires a return value".to_owned()))
        }
        None if matches!(function.return_value.ty, BackendValueType::Unit) => {
            unsafe { LLVMBuildRetVoid(builder) };
            Ok(())
        }
        None => Err(BackendEmitError("non-unit LLVM function requires a return value".to_owned())),
        Some(value_id) => {
            let value = values.get(&value_id).copied().ok_or_else(|| {
                BackendEmitError(format!("LLVM return value {} is unavailable", value_id.0))
            })?;
            if declaration.return_slot {
                let slot = unsafe { llvm_sys::core::LLVMGetParam(declaration.value, 0) };
                unsafe { llvm_sys::core::LLVMBuildStore(builder, value, slot) };
                unsafe { LLVMBuildRetVoid(builder) };
            } else {
                unsafe { LLVMBuildRet(builder, value) };
            }
            Ok(())
        }
    }
}
