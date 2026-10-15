mod address;

use super::super::super::BackendEmitError;
use super::super::super::{BackendCleanupBridge, BackendCleanupOperand};
use super::super::declarations::FunctionDeclaration;
use llvm_sys::core::{LLVMBuildCall2, LLVMConstInt};
use llvm_sys::prelude::{LLVMBuilderRef, LLVMContextRef, LLVMTypeRef, LLVMValueRef};
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
    let symbol = operand.performance_symbol.as_ref().map_or_else(
        || bridge_symbol(operand.bridge).to_owned(),
        |symbol| symbol.as_str().to_owned(),
    );
    let declaration = declarations.get(&symbol).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM cleanup bridge `{symbol}` is unavailable"))
    })?;
    let value = values.get(&operand.value).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM cleanup value {} is unavailable", operand.value.0))
    })?;
    let value = address::cleanup_argument(context, builder, operand.clone(), value)?;
    let mut arguments = vec![value];
    append_bridge_arguments(context, pointer_width, operand.bridge, &mut arguments)?;
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

fn append_bridge_arguments(
    context: LLVMContextRef,
    pointer_width: u8,
    bridge: BackendCleanupBridge,
    arguments: &mut Vec<LLVMValueRef>,
) -> Result<(), BackendEmitError> {
    let (size, alignment) = match bridge {
        BackendCleanupBridge::EnumDrop { size } => (Some(size), None),
        BackendCleanupBridge::AggregateDrop { size, alignment } => (Some(size), Some(alignment)),
        _ => (None, None),
    };
    let Some(size) = size else {
        return Ok(());
    };
    let integer_type = pointer_integer_type(context, pointer_width)?;
    arguments.push(unsafe { LLVMConstInt(integer_type, u64::from(size), 0) });
    if let Some(alignment) = alignment {
        arguments.push(unsafe { LLVMConstInt(integer_type, u64::from(alignment), 0) });
    }
    Ok(())
}

fn pointer_integer_type(
    context: LLVMContextRef,
    pointer_width: u8,
) -> Result<LLVMTypeRef, BackendEmitError> {
    match pointer_width {
        32 => Ok(unsafe { llvm_sys::core::LLVMInt32TypeInContext(context) }),
        64 => Ok(unsafe { llvm_sys::core::LLVMInt64TypeInContext(context) }),
        width => Err(BackendEmitError(format!(
            "LLVM cleanup bridge does not support pointer width {width}"
        ))),
    }
}

fn bridge_symbol(bridge: BackendCleanupBridge) -> &'static str {
    match bridge {
        BackendCleanupBridge::BufferDrop => crate::runtime::BUFFER_DROP_SYMBOL,
        BackendCleanupBridge::RegionDrop => crate::runtime::REGION_DROP_SYMBOL,
        BackendCleanupBridge::EnumDrop { .. } => crate::runtime::ENUM_DROP_SYMBOL,
        BackendCleanupBridge::AggregateDrop { .. } => crate::runtime::AGGREGATE_DROP_SYMBOL,
        BackendCleanupBridge::PerformanceDrop => "",
    }
}
