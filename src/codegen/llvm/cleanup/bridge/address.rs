use super::super::super::super::{BackendCleanupArgument, BackendCleanupOperand, BackendEmitError};
use llvm_sys::core::{LLVMBuildGEP2, LLVMConstInt, LLVMInt8TypeInContext};
use llvm_sys::prelude::{LLVMBuilderRef, LLVMContextRef, LLVMValueRef};
use std::ffi::CString;

pub(super) fn cleanup_argument(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    operand: BackendCleanupOperand,
    value: LLVMValueRef,
) -> Result<LLVMValueRef, BackendEmitError> {
    match operand.argument {
        BackendCleanupArgument::Value | BackendCleanupArgument::Address => Ok(value),
        BackendCleanupArgument::FieldAddress => {
            let field = operand.field.ok_or_else(|| {
                BackendEmitError("LLVM field cleanup requires a field sublayout".to_owned())
            })?;
            let index = unsafe {
                LLVMConstInt(
                    llvm_sys::core::LLVMInt32TypeInContext(context),
                    field.offset as u64,
                    0,
                )
            };
            let name = CString::new("cleanup_field")
                .map_err(|_| BackendEmitError("LLVM cleanup field name contains NUL".to_owned()))?;
            let field_address = unsafe {
                LLVMBuildGEP2(
                    builder,
                    LLVMInt8TypeInContext(context),
                    value,
                    &index as *const _ as *mut _,
                    1,
                    name.as_ptr(),
                )
            };
            (!field_address.is_null()).then_some(field_address).ok_or_else(|| {
                BackendEmitError("LLVM failed to compute cleanup field address".to_owned())
            })
        }
    }
}
