use std::collections::BTreeMap;
use std::ffi::CString;

use llvm_sys::core::{LLVMBuildAlloca, LLVMBuildCall2, LLVMBuildLoad2, LLVMConstInt};
use llvm_sys::prelude::{LLVMBuilderRef, LLVMValueRef};

use super::super::{BackendEmitError, BackendValueId};
use super::declarations::FunctionDeclaration;

struct CallResultContext<'a> {
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    declarations: &'a BTreeMap<String, FunctionDeclaration>,
}

pub(super) fn emit_call(
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    callee: &str,
    argument_ids: &[BackendValueId],
    result: Option<BackendValueId>,
    declarations: &BTreeMap<String, FunctionDeclaration>,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<(), BackendEmitError> {
    let declaration = declarations
        .get(callee)
        .copied()
        .ok_or_else(|| BackendEmitError(format!("LLVM callee `{callee}` is unavailable")))?;
    validate_argument_count(callee, argument_ids, declaration.parameter_count)?;
    let mut arguments = collect_arguments(argument_ids, values)?;
    let return_slot = prepare_return_slot(builder, declaration, result, &mut arguments)?;
    let call = build_call(builder, declaration, result, &mut arguments)?;
    record_call_result(
        CallResultContext { context, builder, declarations },
        declaration,
        result,
        return_slot,
        call,
        values,
    )
}

fn validate_argument_count(
    callee: &str,
    arguments: &[BackendValueId],
    expected: usize,
) -> Result<(), BackendEmitError> {
    if arguments.len() == expected {
        return Ok(());
    }
    Err(BackendEmitError(format!(
        "LLVM call `{callee}` expects {expected} arguments, got {}",
        arguments.len()
    )))
}

fn collect_arguments(
    ids: &[BackendValueId],
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<Vec<LLVMValueRef>, BackendEmitError> {
    ids.iter()
        .map(|id| {
            values
                .get(id)
                .copied()
                .ok_or_else(|| BackendEmitError(format!("LLVM call value {} is unavailable", id.0)))
        })
        .collect()
}

fn prepare_return_slot(
    builder: LLVMBuilderRef,
    declaration: FunctionDeclaration,
    result: Option<BackendValueId>,
    arguments: &mut Vec<LLVMValueRef>,
) -> Result<Option<LLVMValueRef>, BackendEmitError> {
    if !declaration.return_slot {
        return Ok(None);
    }
    let result = result.ok_or_else(|| {
        BackendEmitError("LLVM return-slot call requires a result value".to_owned())
    })?;
    let slot_name = CString::new(format!("slot{}", result.0))
        .map_err(|_| BackendEmitError("LLVM return slot name contains NUL".to_owned()))?;
    let slot = unsafe { LLVMBuildAlloca(builder, declaration.return_type, slot_name.as_ptr()) };
    arguments.insert(0, slot);
    Ok(Some(slot))
}

fn build_call(
    builder: LLVMBuilderRef,
    declaration: FunctionDeclaration,
    result: Option<BackendValueId>,
    arguments: &mut [LLVMValueRef],
) -> Result<LLVMValueRef, BackendEmitError> {
    let call_name = if declaration.returns_void {
        CString::new("")
    } else {
        CString::new(result.map(|id| format!("call{}", id.0)).unwrap_or_default())
    }
    .map_err(|_| BackendEmitError("LLVM call name contains NUL".to_owned()))?;
    let call = unsafe {
        LLVMBuildCall2(
            builder,
            declaration.function_type,
            declaration.value,
            arguments.as_mut_ptr(),
            arguments.len() as u32,
            call_name.as_ptr(),
        )
    };
    (!call.is_null())
        .then_some(call)
        .ok_or_else(|| BackendEmitError("LLVM failed to build call".to_owned()))
}

fn record_call_result(
    result_context: CallResultContext<'_>,
    declaration: FunctionDeclaration,
    result: Option<BackendValueId>,
    return_slot: Option<LLVMValueRef>,
    call: LLVMValueRef,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<(), BackendEmitError> {
    let Some(result) = result else { return Ok(()) };
    if declaration.runtime_pointer_return {
        record_runtime_pointer_result(&result_context, declaration, result, call, values)?;
        return Ok(());
    }
    if let Some(slot) = return_slot {
        let name = CString::new(format!("result{}", result.0))
            .map_err(|_| BackendEmitError("LLVM result name contains NUL".to_owned()))?;
        let value = unsafe {
            LLVMBuildLoad2(result_context.builder, declaration.return_type, slot, name.as_ptr())
        };
        if value.is_null() {
            return Err(BackendEmitError("LLVM failed to load return slot".to_owned()));
        }
        values.insert(result, value);
    } else if declaration.returns_void {
        return Err(BackendEmitError("LLVM call result cannot be void".to_owned()));
    } else {
        values.insert(result, call);
    }
    Ok(())
}

fn record_runtime_pointer_result(
    result_context: &CallResultContext<'_>,
    declaration: FunctionDeclaration,
    result: BackendValueId,
    call: LLVMValueRef,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<(), BackendEmitError> {
    let enum_drop =
        result_context.declarations.get(crate::runtime::ENUM_DROP_SYMBOL).copied().ok_or_else(
            || BackendEmitError("LLVM runtime result cleanup bridge is unavailable".to_owned()),
        )?;
    let slot_name = CString::new(format!("runtime.result{}", result.0))
        .map_err(|_| BackendEmitError("LLVM runtime result name contains NUL".to_owned()))?;
    let value = unsafe {
        LLVMBuildLoad2(result_context.builder, declaration.return_type, call, slot_name.as_ptr())
    };
    if value.is_null() {
        return Err(BackendEmitError("LLVM failed to load runtime result".to_owned()));
    }
    let size = declaration.runtime_return_size.ok_or_else(|| {
        BackendEmitError("LLVM runtime result has no known storage size".to_owned())
    })?;
    let integer_type = unsafe { llvm_sys::core::LLVMInt64TypeInContext(result_context.context) };
    let mut arguments = [call, unsafe { LLVMConstInt(integer_type, u64::from(size), 0) }];
    let cleanup = unsafe {
        LLVMBuildCall2(
            result_context.builder,
            enum_drop.function_type,
            enum_drop.value,
            arguments.as_mut_ptr(),
            arguments.len() as u32,
            c"".as_ptr(),
        )
    };
    if cleanup.is_null() {
        return Err(BackendEmitError("LLVM failed to release runtime result".to_owned()));
    }
    values.insert(result, value);
    Ok(())
}
