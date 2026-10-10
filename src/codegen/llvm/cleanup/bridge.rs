mod address;

use super::super::super::BackendEmitError;
use super::super::super::{BackendCleanupBridge, BackendCleanupOperand};
use super::super::declarations::FunctionDeclaration;
use llvm_sys::core::{LLVMBuildCall2, LLVMConstInt};
use llvm_sys::prelude::{LLVMBuilderRef, LLVMContextRef, LLVMValueRef};
use std::collections::BTreeMap;
use std::ffi::CString;

pub(super) fn emit_runtime_bridge(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    pointer_width: u8,
    operand: BackendCleanupOperand,
    declarations: &BTreeMap<String, FunctionDeclaration>,
    values: &BTreeMap<super::super::super::BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let symbol = bridge_symbol(operand.bridge);
    let declaration = declarations.get(symbol).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM cleanup bridge `{symbol}` is unavailable"))
    })?;
    let value = values.get(&operand.value).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM cleanup value {} is unavailable", operand.value.0))
    })?;
    let value = address::cleanup_argument(context, builder, operand, value)?;
    let mut arguments = vec![value];
    if let BackendCleanupBridge::EnumDrop { size } = operand.bridge {
        let integer_type = match pointer_width {
            32 => unsafe { llvm_sys::core::LLVMInt32TypeInContext(context) },
            64 => unsafe { llvm_sys::core::LLVMInt64TypeInContext(context) },
            width => {
                return Err(BackendEmitError(format!(
                    "LLVM cleanup bridge does not support pointer width {width}"
                )));
            }
        };
        arguments.push(unsafe { LLVMConstInt(integer_type, u64::from(size), 0) });
    }
    let name = CString::new(if declaration.returns_void { "" } else { "cleanup_call" })
        .map_err(|_| BackendEmitError("LLVM cleanup call name contains NUL".to_owned()))?;
    let call = unsafe {
        LLVMBuildCall2(
            builder,
            declaration.function_type,
            declaration.value,
            arguments.as_mut_ptr(),
            arguments.len() as u32,
            name.as_ptr(),
        )
    };
    if call.is_null() {
        return Err(BackendEmitError("LLVM failed to build cleanup bridge call".to_owned()));
    }
    Ok(false)
}

fn bridge_symbol(bridge: BackendCleanupBridge) -> &'static str {
    match bridge {
        BackendCleanupBridge::BufferDrop => crate::runtime::BUFFER_DROP_SYMBOL,
        BackendCleanupBridge::RegionDrop => crate::runtime::REGION_DROP_SYMBOL,
        BackendCleanupBridge::EnumDrop { .. } => crate::runtime::ENUM_DROP_SYMBOL,
    }
}
