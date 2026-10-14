use std::ffi::CString;

use llvm_sys::core::{
    LLVMAddFunction, LLVMFunctionType, LLVMGetParam, LLVMPointerTypeInContext,
    LLVMSetParamAlignment, LLVMVoidTypeInContext,
};
use llvm_sys::prelude::{LLVMContextRef, LLVMModuleRef, LLVMTypeRef, LLVMValueRef};

use super::super::{
    BackendCleanupBridge, BackendEmitError, BackendParameter, BackendPassingMode, BackendReturn,
    BackendReturnMode, BackendValueType,
};
use super::values::llvm_type;

#[derive(Clone, Copy)]
pub(super) struct FunctionDeclaration {
    pub(super) value: LLVMValueRef,
    pub(super) function_type: LLVMTypeRef,
    pub(super) return_type: LLVMTypeRef,
    pub(super) parameter_count: usize,
    pub(super) return_slot: bool,
    pub(super) runtime_pointer_return: bool,
    pub(super) runtime_return_size: Option<u32>,
    pub(super) return_alignment: Option<u32>,
    pub(super) borrowed_view_return: bool,
    pub(super) returns_void: bool,
}

struct ReturnAbi {
    slot_type: LLVMTypeRef,
    return_type: LLVMTypeRef,
    return_slot: bool,
    borrowed_view_return: bool,
    size: Option<u32>,
    alignment: Option<u32>,
    returns_void: bool,
}

pub(super) fn declare_function(
    context: LLVMContextRef,
    module: LLVMModuleRef,
    symbol: &str,
    parameters: &[BackendParameter],
    return_value: &BackendReturn,
    runtime_pointer_return: bool,
    hosted_entry_return: bool,
) -> Result<FunctionDeclaration, BackendEmitError> {
    let abi = return_abi(context, return_value, runtime_pointer_return, hosted_entry_return)?;
    let mut parameter_types = parameters
        .iter()
        .map(|parameter| parameter_type(context, parameter))
        .collect::<Result<Vec<_>, _>>()?;
    if abi.return_slot {
        parameter_types.insert(0, unsafe { LLVMPointerTypeInContext(context, 0) });
    }
    let function_type = unsafe {
        LLVMFunctionType(
            abi.return_type,
            parameter_types.as_mut_ptr(),
            parameter_types.len() as u32,
            0,
        )
    };
    let function_name = CString::new(symbol)
        .map_err(|_| BackendEmitError("LLVM function symbol contains NUL".to_owned()))?;
    let value = unsafe { LLVMAddFunction(module, function_name.as_ptr(), function_type) };
    if value.is_null() {
        return Err(BackendEmitError(format!("LLVM rejected function `{symbol}`")));
    }
    apply_parameter_alignments(value, parameters, return_value, abi.return_slot);
    Ok(FunctionDeclaration {
        value,
        function_type,
        return_type: abi.slot_type,
        parameter_count: parameters.len(),
        return_slot: abi.return_slot,
        runtime_pointer_return,
        runtime_return_size: abi.size,
        return_alignment: abi.alignment,
        borrowed_view_return: abi.borrowed_view_return,
        returns_void: abi.returns_void,
    })
}

fn return_abi(
    context: LLVMContextRef,
    return_value: &BackendReturn,
    runtime_pointer_return: bool,
    hosted_entry_return: bool,
) -> Result<ReturnAbi, BackendEmitError> {
    let borrowed_layout = match return_value.ty {
        BackendValueType::NichePointerOption { payload_layout, .. } => payload_layout,
        _ => None,
    };
    let borrowed = borrowed_layout.is_some();
    let return_slot = borrowed
        || (!runtime_pointer_return
            && matches!(return_value.passing, BackendReturnMode::ReturnSlot { .. }));
    let value_type = llvm_type(context, return_value.ty)?;
    let slot_type = borrowed_layout
        .map(|layout| llvm_type(context, BackendValueType::Aggregate { layout }))
        .transpose()?
        .unwrap_or(value_type);
    let return_type = if hosted_entry_return {
        unsafe { llvm_sys::core::LLVMInt32TypeInContext(context) }
    } else if borrowed {
        unsafe { LLVMPointerTypeInContext(context, 0) }
    } else if return_slot {
        unsafe { LLVMVoidTypeInContext(context) }
    } else if runtime_pointer_return {
        unsafe { LLVMPointerTypeInContext(context, 0) }
    } else {
        value_type
    };
    let (size, alignment) = match borrowed_layout {
        Some(layout) => (Some(layout.size), Some(layout.alignment)),
        None => match return_value.passing {
            BackendReturnMode::ReturnSlot { layout } => (Some(layout.size), Some(layout.alignment)),
            _ => (None, None),
        },
    };
    Ok(ReturnAbi {
        slot_type,
        return_type,
        return_slot,
        borrowed_view_return: borrowed,
        size,
        alignment,
        returns_void: matches!(return_value.ty, BackendValueType::Unit)
            || (return_slot && !borrowed),
    })
}

pub(super) fn declare_cleanup_bridge(
    context: LLVMContextRef,
    module: LLVMModuleRef,
    bridge: BackendCleanupBridge,
    pointer_width: u8,
) -> Result<FunctionDeclaration, BackendEmitError> {
    let (symbol, parameters, return_value) = cleanup_signature(bridge, pointer_width);
    declare_function(context, module, symbol, &parameters, &return_value, false, false)
}

fn cleanup_signature(
    bridge: BackendCleanupBridge,
    pointer_width: u8,
) -> (&'static str, Vec<BackendParameter>, BackendReturn) {
    match bridge {
        BackendCleanupBridge::BufferDrop => (
            crate::runtime::BUFFER_DROP_SYMBOL,
            vec![BackendParameter {
                name: "handle".to_owned(),
                role: crate::ast::Role::Erg,
                ty: BackendValueType::Pointer,
                passing: BackendPassingMode::Direct,
            }],
            BackendReturn {
                role: None,
                ty: BackendValueType::Unit,
                passing: BackendReturnMode::Direct,
            },
        ),
        BackendCleanupBridge::RegionDrop => (
            crate::runtime::REGION_DROP_SYMBOL,
            vec![BackendParameter {
                name: "region".to_owned(),
                role: crate::ast::Role::Erg,
                ty: BackendValueType::Pointer,
                passing: BackendPassingMode::Direct,
            }],
            BackendReturn {
                role: None,
                ty: BackendValueType::Integer { signed: true, width: 32 },
                passing: BackendReturnMode::Direct,
            },
        ),
        BackendCleanupBridge::EnumDrop { .. } => (
            crate::runtime::ENUM_DROP_SYMBOL,
            vec![
                BackendParameter {
                    name: "pointer".to_owned(),
                    role: crate::ast::Role::Erg,
                    ty: BackendValueType::Pointer,
                    passing: BackendPassingMode::Direct,
                },
                BackendParameter {
                    name: "size".to_owned(),
                    role: crate::ast::Role::Erg,
                    ty: BackendValueType::Integer { signed: false, width: pointer_width },
                    passing: BackendPassingMode::Direct,
                },
            ],
            BackendReturn {
                role: None,
                ty: BackendValueType::Unit,
                passing: BackendReturnMode::Direct,
            },
        ),
        BackendCleanupBridge::AggregateDrop { .. } => (
            crate::runtime::AGGREGATE_DROP_SYMBOL,
            vec![
                BackendParameter {
                    name: "pointer".to_owned(),
                    role: crate::ast::Role::Erg,
                    ty: BackendValueType::Pointer,
                    passing: BackendPassingMode::Direct,
                },
                BackendParameter {
                    name: "size".to_owned(),
                    role: crate::ast::Role::Erg,
                    ty: BackendValueType::Integer { signed: false, width: pointer_width },
                    passing: BackendPassingMode::Direct,
                },
                BackendParameter {
                    name: "alignment".to_owned(),
                    role: crate::ast::Role::Erg,
                    ty: BackendValueType::Integer { signed: false, width: pointer_width },
                    passing: BackendPassingMode::Direct,
                },
            ],
            BackendReturn {
                role: None,
                ty: BackendValueType::Unit,
                passing: BackendReturnMode::Direct,
            },
        ),
        BackendCleanupBridge::PerformanceDrop => performance_cleanup_signature(),
    }
}

fn performance_cleanup_signature() -> (&'static str, Vec<BackendParameter>, BackendReturn) {
    (
        "",
        vec![BackendParameter {
            name: "value".to_owned(),
            role: crate::ast::Role::Erg,
            ty: BackendValueType::Pointer,
            passing: BackendPassingMode::Direct,
        }],
        BackendReturn {
            role: None,
            ty: BackendValueType::Unit,
            passing: BackendReturnMode::Direct,
        },
    )
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
        BackendPassingMode::ExclusiveScalarLoan { .. } => {
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
            BackendPassingMode::ExclusiveScalarLoan { .. } => None,
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
