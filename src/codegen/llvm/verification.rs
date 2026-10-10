use std::ffi::CStr;
use std::ptr;

use llvm_sys::analysis::{LLVMVerifierFailureAction, LLVMVerifyModule};
use llvm_sys::core::{LLVMDisposeMessage, LLVMPrintModuleToString};
use llvm_sys::prelude::LLVMModuleRef;

use super::super::BackendEmitError;

pub(super) fn print_module(module: LLVMModuleRef) -> Result<String, BackendEmitError> {
    let printed = unsafe { LLVMPrintModuleToString(module) };
    if printed.is_null() {
        return Err(BackendEmitError("LLVM failed to print the module".to_owned()));
    }
    let text = unsafe { CStr::from_ptr(printed) }
        .to_str()
        .map_err(|_| BackendEmitError("LLVM produced non-UTF-8 IR".to_owned()))?
        .to_owned();
    unsafe { LLVMDisposeMessage(printed) };
    Ok(text)
}

pub(super) fn verify_module(module: LLVMModuleRef) -> Result<(), BackendEmitError> {
    let mut message = ptr::null_mut();
    let failed = unsafe {
        LLVMVerifyModule(module, LLVMVerifierFailureAction::LLVMReturnStatusAction, &mut message)
            != 0
    };
    if !failed {
        return Ok(());
    }
    let diagnostic = if message.is_null() {
        "LLVM module verification failed".to_owned()
    } else {
        let text = unsafe { CStr::from_ptr(message) }.to_string_lossy().into_owned();
        unsafe { LLVMDisposeMessage(message) };
        text
    };
    Err(BackendEmitError(format!("LLVM module verification failed: {diagnostic}")))
}
