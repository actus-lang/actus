use std::collections::BTreeMap;
use std::ffi::CString;

use llvm_sys::core::LLVMBuildGlobalString;
use llvm_sys::prelude::{LLVMBuilderRef, LLVMValueRef};

use super::super::{BackendEmitError, BackendValueId};

pub(super) fn emit_string_literal(
    builder: LLVMBuilderRef,
    result: BackendValueId,
    value: &str,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let value = CString::new(value)
        .map_err(|_| BackendEmitError("LLVM string literal contains an embedded NUL".to_owned()))?;
    let name = CString::new(format!("actus.string.{}", result.0))
        .map_err(|_| BackendEmitError("LLVM string literal name contains NUL".to_owned()))?;
    let pointer = unsafe { LLVMBuildGlobalString(builder, value.as_ptr(), name.as_ptr()) };
    if pointer.is_null() {
        return Err(BackendEmitError("LLVM string literal emission failed".to_owned()));
    }
    values.insert(result, pointer);
    Ok(false)
}
