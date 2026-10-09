use std::collections::BTreeMap;

use llvm_sys::core::{LLVMBuildRet, LLVMBuildRetVoid};
use llvm_sys::prelude::{LLVMBuilderRef, LLVMValueRef};

use super::super::{BackendEmitError, BackendFunction, BackendValueId, BackendValueType};

pub(super) fn emit_return(
    builder: LLVMBuilderRef,
    function: &BackendFunction,
    value_id: Option<BackendValueId>,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<(), BackendEmitError> {
    match value_id {
        None if matches!(function.return_value.ty, BackendValueType::Unit) => {
            unsafe { LLVMBuildRetVoid(builder) };
            Ok(())
        }
        None => Err(BackendEmitError("non-unit LLVM function requires a return value".to_owned())),
        Some(value_id) => {
            let value = values.get(&value_id).copied().ok_or_else(|| {
                BackendEmitError(format!("LLVM return value {} is unavailable", value_id.0))
            })?;
            unsafe { LLVMBuildRet(builder, value) };
            Ok(())
        }
    }
}
