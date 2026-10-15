use std::collections::BTreeMap;

use llvm_sys::core::{
    LLVMAppendBasicBlockInContext, LLVMBuildAlloca, LLVMBuildCondBr, LLVMBuildICmp, LLVMBuildLoad2,
    LLVMBuildMemMove, LLVMBuildRet, LLVMBuildRetVoid, LLVMBuildStore, LLVMConstInt, LLVMConstNull,
    LLVMGetTypeKind, LLVMInt32TypeInContext, LLVMInt64TypeInContext, LLVMPositionBuilderAtEnd,
    LLVMSetAlignment, LLVMTypeOf,
};
use llvm_sys::prelude::{LLVMBuilderRef, LLVMContextRef, LLVMValueRef};
use llvm_sys::{LLVMIntPredicate, LLVMTypeKind};

use super::super::{
    BackendEmitError, BackendFunction, BackendReturnMode, BackendValueId, BackendValueType,
};
use super::declarations::FunctionDeclaration;

pub(super) fn emit_return(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    function: &BackendFunction,
    declaration: FunctionDeclaration,
    value_id: Option<BackendValueId>,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<(), BackendEmitError> {
    match value_id {
        None if declaration.return_slot => {
            Err(BackendEmitError("LLVM return-slot function requires a return value".to_owned()))
        }
        None if function.hosted_entry_return => {
            unsafe {
                LLVMBuildRet(builder, LLVMConstInt(LLVMInt32TypeInContext(context), 0, 0));
            }
            Ok(())
        }
        None if matches!(function.return_value.ty, BackendValueType::Unit) => {
            unsafe { LLVMBuildRetVoid(builder) };
            Ok(())
        }
        None => Err(BackendEmitError("non-unit LLVM function requires a return value".to_owned())),
        Some(value_id) => {
            emit_present_return(context, builder, function, declaration, value_id, values)
        }
    }
}

fn emit_present_return(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    function: &BackendFunction,
    declaration: FunctionDeclaration,
    value_id: BackendValueId,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<(), BackendEmitError> {
    let value = values.get(&value_id).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM return value {} is unavailable", value_id.0))
    })?;
    if !declaration.return_slot {
        let value = direct_return_value(context, builder, function, declaration, value)?;
        unsafe { LLVMBuildRet(builder, value) };
        return Ok(());
    }
    let slot = unsafe { llvm_sys::core::LLVMGetParam(declaration.value, 0) };
    if declaration.borrowed_view_return {
        return emit_borrowed_view_return(context, builder, declaration, slot, value);
    }
    let size = declaration
        .runtime_return_size
        .ok_or_else(|| BackendEmitError("LLVM return slot has no known storage size".to_owned()))?;
    let alignment = declaration
        .return_alignment
        .ok_or_else(|| BackendEmitError("LLVM return slot has no known alignment".to_owned()))?;
    let source = return_slot_source(context, builder, declaration, value, function)?;
    let copied = unsafe {
        LLVMBuildMemMove(
            builder,
            slot,
            alignment,
            source,
            alignment,
            LLVMConstInt(LLVMInt64TypeInContext(context), u64::from(size), 0),
        )
    };
    if copied.is_null() {
        return Err(BackendEmitError("LLVM failed to copy return slot".to_owned()));
    }
    unsafe { LLVMBuildRetVoid(builder) };
    Ok(())
}

fn emit_borrowed_view_return(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    declaration: FunctionDeclaration,
    slot: LLVMValueRef,
    value: LLVMValueRef,
) -> Result<(), BackendEmitError> {
    let null_block = unsafe {
        LLVMAppendBasicBlockInContext(context, declaration.value, c"borrowed.none".as_ptr())
    };
    let some_block = unsafe {
        LLVMAppendBasicBlockInContext(context, declaration.value, c"borrowed.some".as_ptr())
    };
    let null = unsafe { LLVMConstNull(LLVMTypeOf(value)) };
    let is_null = unsafe {
        LLVMBuildICmp(
            builder,
            LLVMIntPredicate::LLVMIntEQ,
            value,
            null,
            c"borrowed.is_null".as_ptr(),
        )
    };
    unsafe { LLVMBuildCondBr(builder, is_null, null_block, some_block) };
    unsafe { LLVMPositionBuilderAtEnd(builder, null_block) };
    unsafe { LLVMBuildRet(builder, null) };
    unsafe { LLVMPositionBuilderAtEnd(builder, some_block) };
    let size = declaration
        .runtime_return_size
        .ok_or_else(|| BackendEmitError("borrowed view return has no payload size".to_owned()))?;
    let alignment = declaration.return_alignment.ok_or_else(|| {
        BackendEmitError("borrowed view return has no payload alignment".to_owned())
    })?;
    let copied = unsafe {
        LLVMBuildMemMove(
            builder,
            slot,
            alignment,
            value,
            alignment,
            LLVMConstInt(LLVMInt64TypeInContext(context), u64::from(size), 0),
        )
    };
    if copied.is_null() {
        return Err(BackendEmitError("LLVM failed to copy borrowed view return".to_owned()));
    }
    unsafe { LLVMBuildRet(builder, slot) };
    Ok(())
}

fn return_slot_source(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    declaration: FunctionDeclaration,
    value: LLVMValueRef,
    function: &BackendFunction,
) -> Result<LLVMValueRef, BackendEmitError> {
    let value_type = unsafe { LLVMTypeOf(value) };
    if unsafe { LLVMGetTypeKind(value_type) } == LLVMTypeKind::LLVMPointerTypeKind {
        return Ok(value);
    }
    let BackendReturnMode::ReturnSlot { layout } = function.return_value.passing else {
        return Err(BackendEmitError(
            "LLVM return-slot source has an incompatible return mode".to_owned(),
        ));
    };
    let temporary = unsafe {
        LLVMBuildAlloca(builder, declaration.return_type, c"return.slot.source".as_ptr())
    };
    if temporary.is_null() {
        return Err(BackendEmitError("LLVM failed to allocate return-slot source".to_owned()));
    }
    unsafe { LLVMSetAlignment(temporary, layout.alignment) };
    if unsafe { LLVMBuildStore(builder, value, temporary) }.is_null() {
        return Err(BackendEmitError("LLVM failed to store return-slot source".to_owned()));
    }
    let _ = context;
    Ok(temporary)
}

fn direct_return_value(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    function: &BackendFunction,
    declaration: FunctionDeclaration,
    value: LLVMValueRef,
) -> Result<LLVMValueRef, BackendEmitError> {
    let requires_aggregate_value = matches!(
        function.return_value.passing,
        BackendReturnMode::Direct | BackendReturnMode::DirectAggregate { .. }
    ) && matches!(
        function.return_value.ty,
        BackendValueType::Aggregate { .. } | BackendValueType::Enum { .. }
    );
    if !requires_aggregate_value {
        return Ok(value);
    }
    let value_type = unsafe { LLVMTypeOf(value) };
    if unsafe { LLVMGetTypeKind(value_type) } != LLVMTypeKind::LLVMPointerTypeKind {
        return Ok(value);
    }
    let loaded = unsafe {
        LLVMBuildLoad2(builder, declaration.return_type, value, c"direct.aggregate.return".as_ptr())
    };
    if loaded.is_null() {
        return Err(BackendEmitError(
            "LLVM failed to materialize direct aggregate return".to_owned(),
        ));
    }
    let _ = context;
    Ok(loaded)
}
