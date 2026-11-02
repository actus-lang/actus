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

pub(super) fn verify_no_float_ir(module: LLVMModuleRef) -> Result<(), BackendEmitError> {
    let ir = print_module(module)?;
    if ir.split_whitespace().any(is_float_ir_token) {
        return Err(BackendEmitError(
            "LLVM zero-float verification rejected floating-point IR".to_owned(),
        ));
    }
    if ir.lines().any(is_float_helper_symbol) {
        return Err(BackendEmitError(
            "LLVM zero-float verification rejected soft-float helper".to_owned(),
        ));
    }
    Ok(())
}

fn is_float_ir_token(token: &str) -> bool {
    matches!(
        token.trim_matches(|character: char| !character.is_ascii_alphabetic()),
        "float"
            | "double"
            | "fp128"
            | "fneg"
            | "fadd"
            | "fsub"
            | "fmul"
            | "fdiv"
            | "frem"
            | "fptoui"
            | "fptosi"
            | "uitofp"
            | "sitofp"
            | "fptrunc"
            | "fpext"
            | "fcmp"
    )
}

fn is_float_helper_symbol(line: &str) -> bool {
    [
        "__addsf3", "__subsf3", "__mulsf3", "__divsf3", "__gesf2", "__lesf2", "__gtsf2", "__ltsf2",
        "__eqsf2", "__nesf2", "__adddf3", "__subdf3", "__muldf3", "__divdf3", "__gedf2", "__ledf2",
        "__gtdf2", "__ltdf2", "__eqdf2", "__nedf2",
    ]
    .iter()
    .any(|symbol| line.contains(symbol))
        || line.contains("__aeabi_f")
        || line.contains("sf3")
        || line.contains("df3")
}
