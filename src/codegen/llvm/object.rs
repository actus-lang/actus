use std::ffi::{CStr, CString};
use std::path::Path;
use std::ptr;
use std::sync::Once;

use llvm_sys::core::{
    LLVMDisposeMemoryBuffer, LLVMDisposeMessage, LLVMGetBufferSize, LLVMGetBufferStart,
    LLVMSetTarget,
};
use llvm_sys::target_machine::{
    LLVMCodeGenFileType, LLVMCodeGenOptLevel, LLVMCodeModel, LLVMCreateTargetDataLayout,
    LLVMCreateTargetMachine, LLVMDisposeTargetMachine, LLVMGetTargetFromTriple, LLVMRelocMode,
    LLVMTargetHasTargetMachine, LLVMTargetMachineEmitToFile, LLVMTargetMachineEmitToMemoryBuffer,
};
use target_lexicon::BinaryFormat;

use super::super::BackendEmitError;
use super::emission::{ObjectEmissionOptions, ObjectOptimizationLevel};
use crate::target::TargetSpec;

static LLVM_TARGETS_INITIALIZED: Once = Once::new();

pub(super) fn emit_object(
    module: llvm_sys::prelude::LLVMModuleRef,
    target: &TargetSpec,
    output: &Path,
    position_independent: bool,
    options: ObjectEmissionOptions,
) -> Result<(), BackendEmitError> {
    initialize_targets()?;
    let triple = CString::new(target.triple().to_string())
        .map_err(|_| BackendEmitError("LLVM target triple contains NUL".to_owned()))?;
    let output = output
        .to_str()
        .ok_or_else(|| BackendEmitError("LLVM object path is not valid UTF-8".to_owned()))?;
    let output = CString::new(output)
        .map_err(|_| BackendEmitError("LLVM object path contains NUL".to_owned()))?;
    let machine = TargetMachine::new(
        triple.as_c_str(),
        position_independent,
        target.object_format == BinaryFormat::Elf,
        options,
    )?;
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

pub(super) fn emit_object_bytes(
    module: llvm_sys::prelude::LLVMModuleRef,
    target: &TargetSpec,
    position_independent: bool,
    options: ObjectEmissionOptions,
) -> Result<Vec<u8>, BackendEmitError> {
    initialize_targets()?;
    let triple = CString::new(target.triple().to_string())
        .map_err(|_| BackendEmitError("LLVM target triple contains NUL".to_owned()))?;
    let machine = TargetMachine::new(
        triple.as_c_str(),
        position_independent,
        target.object_format == BinaryFormat::Elf,
        options,
    )?;
    let data_layout = unsafe { LLVMCreateTargetDataLayout(machine.raw) };
    if data_layout.is_null() {
        return Err(BackendEmitError("LLVM failed to create target data layout".to_owned()));
    }
    unsafe { LLVMSetTarget(module, triple.as_ptr()) };
    unsafe { llvm_sys::target::LLVMSetModuleDataLayout(module, data_layout) };
    let mut error = ptr::null_mut();
    let mut buffer = ptr::null_mut();
    let failed = unsafe {
        LLVMTargetMachineEmitToMemoryBuffer(
            machine.raw,
            module,
            LLVMCodeGenFileType::LLVMObjectFile,
            &mut error,
            &mut buffer,
        ) != 0
    };
    unsafe { llvm_sys::target::LLVMDisposeTargetData(data_layout) };
    if failed {
        if !buffer.is_null() {
            unsafe { LLVMDisposeMemoryBuffer(buffer) };
        }
        return Err(BackendEmitError(format_llvm_error(error, "LLVM object emission failed")));
    }
    if buffer.is_null() {
        return Err(BackendEmitError("LLVM returned no object memory buffer".to_owned()));
    }
    let size = unsafe { LLVMGetBufferSize(buffer) };
    let start = unsafe { LLVMGetBufferStart(buffer) };
    if size != 0 && start.is_null() {
        unsafe { LLVMDisposeMemoryBuffer(buffer) };
        return Err(BackendEmitError("LLVM returned an invalid object memory buffer".to_owned()));
    }
    let bytes = unsafe { std::slice::from_raw_parts(start.cast::<u8>(), size).to_vec() };
    unsafe { LLVMDisposeMemoryBuffer(buffer) };
    Ok(bytes)
}

fn initialize_targets() -> Result<(), BackendEmitError> {
    LLVM_TARGETS_INITIALIZED.call_once(|| {
        unsafe {
            llvm_sys::target::LLVM_InitializeAllTargetInfos();
            llvm_sys::target::LLVM_InitializeAllTargets();
            llvm_sys::target::LLVM_InitializeAllTargetMCs();
            llvm_sys::target::LLVM_InitializeAllAsmPrinters();
        }
        // LLVM exposes this control as a process-global cl::opt rather than a
        // TargetMachine option. Set it before any TargetMachine is created so
        // every LLVM emission in this process has the same frame accounting.
        super::shim::disable_x86_call_frame_optimization();
    });
    Ok(())
}

struct TargetMachine {
    raw: llvm_sys::target_machine::LLVMTargetMachineRef,
}

impl TargetMachine {
    fn new(
        triple: &std::ffi::CStr,
        position_independent: bool,
        is_elf: bool,
        options: super::emission::ObjectEmissionOptions,
    ) -> Result<Self, BackendEmitError> {
        let mut llvm_target = ptr::null_mut();
        let mut error = ptr::null_mut();
        let failed =
            unsafe { LLVMGetTargetFromTriple(triple.as_ptr(), &mut llvm_target, &mut error) != 0 };
        if failed {
            return Err(BackendEmitError(format_llvm_error(error, "LLVM target lookup failed")));
        }
        if llvm_target.is_null() || unsafe { LLVMTargetHasTargetMachine(llvm_target) == 0 } {
            return Err(BackendEmitError("LLVM target has no target machine".to_owned()));
        }
        let cpu = CString::new("").expect("empty CPU string cannot contain NUL");
        let features = CString::new("").expect("empty feature string cannot contain NUL");
        let options = if is_elf { options } else { ObjectEmissionOptions::default() };
        let raw = unsafe {
            LLVMCreateTargetMachine(
                llvm_target,
                triple.as_ptr(),
                cpu.as_ptr(),
                features.as_ptr(),
                match options.optimization_level {
                    ObjectOptimizationLevel::None => LLVMCodeGenOptLevel::LLVMCodeGenLevelNone,
                    ObjectOptimizationLevel::Default => {
                        LLVMCodeGenOptLevel::LLVMCodeGenLevelDefault
                    }
                },
                if position_independent {
                    LLVMRelocMode::LLVMRelocPIC
                } else {
                    LLVMRelocMode::LLVMRelocStatic
                },
                LLVMCodeModel::LLVMCodeModelDefault,
            )
        };
        if raw.is_null() {
            return Err(BackendEmitError("LLVM failed to create target machine".to_owned()));
        }
        super::shim::apply(
            raw,
            super::shim::TargetMachineOptions {
                emit_stack_size_section: options.emit_stack_size_section,
                function_sections: options.function_sections,
            },
        );
        Ok(Self { raw })
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
