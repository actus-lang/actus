use std::ffi::CString;

use llvm_sys::core::{
    LLVMAddFunction, LLVMFunctionType, LLVMGetParam, LLVMPointerTypeInContext,
    LLVMSetParamAlignment, LLVMVoidTypeInContext,
};
use llvm_sys::prelude::{LLVMContextRef, LLVMModuleRef, LLVMTypeRef, LLVMValueRef};

use super::super::{
    BackendEmitError, BackendParameter, BackendPassingMode, BackendReturn, BackendReturnMode,
};
use super::values::llvm_type;

#[derive(Clone, Copy)]
pub(super) struct FunctionDeclaration {
    pub(super) value: LLVMValueRef,
    pub(super) function_type: LLVMTypeRef,
    pub(super) return_type: LLVMTypeRef,
    pub(super) parameter_count: usize,
    pub(super) return_slot: bool,
    pub(super) returns_void: bool,
}

pub(super) fn declare_function(
    context: LLVMContextRef,
    module: LLVMModuleRef,
    symbol: &str,
    parameters: &[BackendParameter],
    return_value: &BackendReturn,
) -> Result<FunctionDeclaration, BackendEmitError> {
    let return_slot = matches!(&return_value.passing, BackendReturnMode::ReturnSlot { .. });
    let value_type = llvm_type(context, return_value.ty)?;
    let return_type =
        if return_slot { unsafe { LLVMVoidTypeInContext(context) } } else { value_type };
    let mut parameter_types = parameters
        .iter()
        .map(|parameter| parameter_type(context, parameter))
        .collect::<Result<Vec<_>, _>>()?;
    if return_slot {
        parameter_types.insert(0, unsafe { LLVMPointerTypeInContext(context, 0) });
    }
    let function_type = unsafe {
        LLVMFunctionType(return_type, parameter_types.as_mut_ptr(), parameter_types.len() as u32, 0)
    };
    let function_name = CString::new(symbol)
        .map_err(|_| BackendEmitError("LLVM function symbol contains NUL".to_owned()))?;
    let value = unsafe { LLVMAddFunction(module, function_name.as_ptr(), function_type) };
    if value.is_null() {
        return Err(BackendEmitError(format!("LLVM rejected function `{symbol}`")));
    }
    apply_parameter_alignments(value, parameters, return_value, return_slot);
    Ok(FunctionDeclaration {
        value,
        function_type,
        return_type: value_type,
        parameter_count: parameters.len(),
        return_slot,
        returns_void: matches!(return_value.ty, super::super::BackendValueType::Unit)
            || return_slot,
    })
}

fn parameter_type(
    context: LLVMContextRef,
    parameter: &BackendParameter,
) -> Result<LLVMTypeRef, BackendEmitError> {
    match &parameter.passing {
        BackendPassingMode::Direct | BackendPassingMode::DirectAggregate { .. } => {
            llvm_type(context, parameter.ty)
        }
        BackendPassingMode::Indirect { .. }
        | BackendPassingMode::Borrowed { .. }
        | BackendPassingMode::ExclusiveLoan { .. }
        | BackendPassingMode::OwnershipTransfer { .. }
        | BackendPassingMode::RuntimeBridge { .. } => {
            Ok(unsafe { LLVMPointerTypeInContext(context, 0) })
        }
    }
}

fn apply_parameter_alignments(
    function: LLVMValueRef,
    parameters: &[BackendParameter],
    return_value: &BackendReturn,
    return_slot: bool,
) {
    if let BackendReturnMode::ReturnSlot { layout } = &return_value.passing {
        let value = unsafe { LLVMGetParam(function, 0) };
        unsafe { LLVMSetParamAlignment(value, layout.alignment) };
    }
    for (index, parameter) in parameters.iter().enumerate() {
        let layout = match &parameter.passing {
            BackendPassingMode::Indirect { layout }
            | BackendPassingMode::Borrowed { layout }
            | BackendPassingMode::ExclusiveLoan { layout }
            | BackendPassingMode::OwnershipTransfer { layout }
            | BackendPassingMode::RuntimeBridge { layout } => Some(*layout),
            BackendPassingMode::Direct | BackendPassingMode::DirectAggregate { .. } => None,
        };
        let Some(layout) = layout else {
            continue;
        };
        let physical_index = index + usize::from(return_slot);
        let value = unsafe { LLVMGetParam(function, physical_index as u32) };
        unsafe { LLVMSetParamAlignment(value, layout.alignment) };
    }
}
