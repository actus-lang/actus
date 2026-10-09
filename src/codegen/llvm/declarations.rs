use std::ffi::CString;

use llvm_sys::core::{LLVMAddFunction, LLVMFunctionType};
use llvm_sys::prelude::{LLVMContextRef, LLVMModuleRef, LLVMTypeRef, LLVMValueRef};

use super::super::{BackendEmitError, BackendParameter, BackendReturn, BackendReturnMode};
use super::values::llvm_type;

#[derive(Clone, Copy)]
pub(super) struct FunctionDeclaration {
    pub(super) value: LLVMValueRef,
    pub(super) function_type: LLVMTypeRef,
    pub(super) parameter_count: usize,
    pub(super) returns_void: bool,
}

pub(super) fn declare_function(
    context: LLVMContextRef,
    module: LLVMModuleRef,
    symbol: &str,
    parameters: &[BackendParameter],
    return_value: &BackendReturn,
) -> Result<FunctionDeclaration, BackendEmitError> {
    if !matches!(return_value.passing, BackendReturnMode::Direct) {
        return Err(BackendEmitError("LLVM aggregate return slots are not implemented".to_owned()));
    }
    let return_type = llvm_type(context, return_value.ty)?;
    let mut parameter_types = parameters
        .iter()
        .map(|parameter| llvm_type(context, parameter.ty))
        .collect::<Result<Vec<_>, _>>()?;
    let function_type = unsafe {
        LLVMFunctionType(return_type, parameter_types.as_mut_ptr(), parameter_types.len() as u32, 0)
    };
    let function_name = CString::new(symbol)
        .map_err(|_| BackendEmitError("LLVM function symbol contains NUL".to_owned()))?;
    let value = unsafe { LLVMAddFunction(module, function_name.as_ptr(), function_type) };
    if value.is_null() {
        return Err(BackendEmitError(format!("LLVM rejected function `{symbol}`")));
    }
    Ok(FunctionDeclaration {
        value,
        function_type,
        parameter_count: parameters.len(),
        returns_void: matches!(return_value.ty, super::super::BackendValueType::Unit),
    })
}
