use llvm_sys::core::{
    LLVMContextCreate, LLVMContextDispose, LLVMDisposeModule, LLVMModuleCreateWithNameInContext,
};
use llvm_sys::prelude::{LLVMContextRef, LLVMModuleRef};

use super::super::BackendEmitError;

pub(super) struct LlvmContext {
    pub(super) raw: LLVMContextRef,
}

impl LlvmContext {
    pub(super) fn new() -> Result<Self, BackendEmitError> {
        let raw = unsafe { LLVMContextCreate() };
        (!raw.is_null())
            .then_some(Self { raw })
            .ok_or_else(|| BackendEmitError("LLVM failed to create a context".to_owned()))
    }
}

impl Drop for LlvmContext {
    fn drop(&mut self) {
        unsafe { LLVMContextDispose(self.raw) }
    }
}

pub(super) struct LlvmModule {
    pub(super) raw: LLVMModuleRef,
}

impl LlvmModule {
    pub(super) fn new(
        context: LLVMContextRef,
        name: *const std::ffi::c_char,
    ) -> Result<Self, BackendEmitError> {
        let raw = unsafe { LLVMModuleCreateWithNameInContext(name, context) };
        (!raw.is_null())
            .then_some(Self { raw })
            .ok_or_else(|| BackendEmitError("LLVM failed to create a module".to_owned()))
    }
}

impl Drop for LlvmModule {
    fn drop(&mut self) {
        unsafe { LLVMDisposeModule(self.raw) }
    }
}
