use std::collections::BTreeSet;
use std::ffi::CString;

use llvm_sys::LLVMLinkage;
use llvm_sys::core::{LLVMGetNamedFunction, LLVMSetLinkage};
use llvm_sys::prelude::LLVMModuleRef;

use super::super::BackendEmitError;

/// Gives every function in `local_symbols` internal linkage (ADR-0112).
pub(super) fn localize(
    module: LLVMModuleRef,
    local_symbols: &BTreeSet<String>,
) -> Result<(), BackendEmitError> {
    for symbol in local_symbols {
        let name = CString::new(symbol.as_str())
            .map_err(|_| BackendEmitError("LLVM function symbol contains NUL".to_owned()))?;
        let function = unsafe { LLVMGetNamedFunction(module, name.as_ptr()) };
        if function.is_null() {
            return Err(BackendEmitError(format!(
                "internal compiler error: local symbol `{symbol}` is not defined in its LLVM module"
            )));
        }
        unsafe { LLVMSetLinkage(function, LLVMLinkage::LLVMInternalLinkage) };
    }
    Ok(())
}
