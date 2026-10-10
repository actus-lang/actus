use std::ffi::{CStr, CString};
use std::path::Path;
use std::ptr;

use llvm_sys::core::{LLVMDisposeMessage, LLVMSetTarget};
use llvm_sys::target_machine::{
    LLVMCodeGenFileType, LLVMCodeGenOptLevel, LLVMCodeModel, LLVMCreateTargetDataLayout,
    LLVMCreateTargetMachine, LLVMDisposeTargetMachine, LLVMGetTargetFromTriple, LLVMRelocMode,
    LLVMTargetHasTargetMachine, LLVMTargetMachineEmitToFile,
};

use super::super::BackendEmitError;
use crate::target::TargetSpec;

pub(super) fn emit_object(
    module: llvm_sys::prelude::LLVMModuleRef,
    target: &TargetSpec,
    output: &Path,
) -> Result<(), BackendEmitError> {
    initialize_targets()?;
    let triple = CString::new(target.triple().to_string())
        .map_err(|_| BackendEmitError("LLVM target triple contains NUL".to_owned()))?;
    let output = output
        .to_str()
        .ok_or_else(|| BackendEmitError("LLVM object path is not valid UTF-8".to_owned()))?;
    let output = CString::new(output)
        .map_err(|_| BackendEmitError("LLVM object path contains NUL".to_owned()))?;
    let machine = TargetMachine::new(triple.as_c_str())?;
    let data_layout = unsafe { LLVMCreateTargetDataLayout(machine.raw) };
    if data_layout.is_null() {
        return Err(BackendEmitError("LLVM failed to create target data layout".to_owned()));
    }
    unsafe { LLVMSetTarget(module, triple.as_ptr()) };
    unsafe { llvm_sys::target::LLVMSetModuleDataLayout(module, data_layout) };
    let mut error = ptr::null_mut();
    let failed = unsafe {
        LLVMTargetMachineEmitToFile(
            machine.raw,
            module,
            output.as_ptr().cast_mut(),
            LLVMCodeGenFileType::LLVMObjectFile,
            &mut error,
        ) != 0
    };
    unsafe { llvm_sys::target::LLVMDisposeTargetData(data_layout) };
    if failed {
        return Err(BackendEmitError(format_llvm_error(error, "LLVM object emission failed")));
    }
    Ok(())
}

fn initialize_targets() -> Result<(), BackendEmitError> {
    unsafe {
        llvm_sys::target::LLVM_InitializeAllTargetInfos();
        llvm_sys::target::LLVM_InitializeAllTargets();
        llvm_sys::target::LLVM_InitializeAllTargetMCs();
        llvm_sys::target::LLVM_InitializeAllAsmPrinters();
    }
    Ok(())
}

struct TargetMachine {
    raw: llvm_sys::target_machine::LLVMTargetMachineRef,
}

impl TargetMachine {
    fn new(triple: &std::ffi::CStr) -> Result<Self, BackendEmitError> {
        let mut target = ptr::null_mut();
        let mut error = ptr::null_mut();
        let failed =
            unsafe { LLVMGetTargetFromTriple(triple.as_ptr(), &mut target, &mut error) != 0 };
        if failed {
            return Err(BackendEmitError(format_llvm_error(error, "LLVM target lookup failed")));
        }
        if target.is_null() || unsafe { LLVMTargetHasTargetMachine(target) == 0 } {
            return Err(BackendEmitError("LLVM target has no target machine".to_owned()));
        }
        let cpu = CString::new("").expect("empty CPU string cannot contain NUL");
        let features = CString::new("").expect("empty feature string cannot contain NUL");
        let raw = unsafe {
            LLVMCreateTargetMachine(
                target,
                triple.as_ptr(),
                cpu.as_ptr(),
                features.as_ptr(),
                LLVMCodeGenOptLevel::LLVMCodeGenLevelNone,
                LLVMRelocMode::LLVMRelocStatic,
                LLVMCodeModel::LLVMCodeModelDefault,
            )
        };
        (!raw.is_null())
            .then_some(Self { raw })
            .ok_or_else(|| BackendEmitError("LLVM failed to create target machine".to_owned()))
    }
}

impl Drop for TargetMachine {
    fn drop(&mut self) {
        unsafe { LLVMDisposeTargetMachine(self.raw) }
    }
}

fn format_llvm_error(message: *mut std::ffi::c_char, fallback: &str) -> String {
    if message.is_null() {
        return fallback.to_owned();
    }
    let text = unsafe { CStr::from_ptr(message) }.to_string_lossy().into_owned();
    unsafe { LLVMDisposeMessage(message) };
    format!("{fallback}: {text}")
}
