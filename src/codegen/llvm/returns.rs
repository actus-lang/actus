use std::collections::BTreeMap;

use llvm_sys::core::{
    LLVMBuildMemMove, LLVMBuildRet, LLVMBuildRetVoid, LLVMConstInt, LLVMInt64TypeInContext,
};
use llvm_sys::prelude::{LLVMBuilderRef, LLVMContextRef, LLVMValueRef};

use super::super::{BackendEmitError, BackendFunction, BackendValueId, BackendValueType};
use super::declarations::FunctionDeclaration;

pub(super) fn emit_return(
    context: LLVMContextRef,
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
                let size = declaration.runtime_return_size.ok_or_else(|| {
                    BackendEmitError("LLVM return slot has no known storage size".to_owned())
                })?;
                let alignment = declaration.return_alignment.ok_or_else(|| {
                    BackendEmitError("LLVM return slot has no known alignment".to_owned())
                })?;
                let copied = unsafe {
                    LLVMBuildMemMove(
                        builder,
                        slot,
                        alignment,
                        value,
                        alignment,
                        LLVMConstInt(LLVMInt64TypeInContext(context), u64::from(size), 0),
                    )
                };
                if copied.is_null() {
                    return Err(BackendEmitError("LLVM failed to copy return slot".to_owned()));
                }
                unsafe { LLVMBuildRetVoid(builder) };
            } else {
                unsafe { LLVMBuildRet(builder, value) };
            }
            Ok(())
        }
    }
}
