use super::super::super::super::{
    BackendCleanupArgument, BackendCleanupBridge, BackendCleanupOperand, BackendEmitError,
};
use llvm_sys::core::{
    LLVMBuildGEP2, LLVMBuildLoad2, LLVMConstInt, LLVMInt8TypeInContext, LLVMPointerTypeInContext,
};
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
            if field_address.is_null() {
                return Err(BackendEmitError(
                    "LLVM failed to compute cleanup field address".to_owned(),
                ));
            }
            if operand.bridge != BackendCleanupBridge::BufferDrop {
                return Ok(field_address);
            }
            // A buffer field stores the buffer handle, and the runtime releases
            // the handle itself, not the address of the slot that holds it.
            let handle_name = CString::new("cleanup_buffer_handle").map_err(|_| {
                BackendEmitError("LLVM cleanup handle name contains NUL".to_owned())
            })?;
            let handle = unsafe {
                LLVMBuildLoad2(
                    builder,
                    LLVMPointerTypeInContext(context, 0),
                    field_address,
                    handle_name.as_ptr(),
                )
            };
            (!handle.is_null()).then_some(handle).ok_or_else(|| {
                BackendEmitError("LLVM failed to load the cleanup buffer handle".to_owned())
            })
        }
    }
}
