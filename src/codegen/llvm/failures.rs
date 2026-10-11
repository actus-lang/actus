use std::collections::BTreeMap;

use llvm_sys::LLVMTypeKind::{LLVMIntegerTypeKind, LLVMPointerTypeKind};
use llvm_sys::core::{
    LLVMBuildAlloca, LLVMBuildGEP2, LLVMBuildLoad2, LLVMBuildRetVoid, LLVMBuildStore, LLVMConstInt,
    LLVMConstNull, LLVMGetIntTypeWidth, LLVMGetParam, LLVMGetTypeKind, LLVMInt8TypeInContext,
    LLVMInt16TypeInContext, LLVMInt32TypeInContext, LLVMInt64TypeInContext, LLVMSetAlignment,
    LLVMTypeOf,
};
use llvm_sys::prelude::{LLVMBuilderRef, LLVMValueRef};

use super::super::{
    BackendEmitError, BackendEnumLayout, BackendFunction, BackendLayout, BackendValueId,
    BackendValueType,
};
use super::declarations::FunctionDeclaration;

pub(super) fn emit_failure(
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    function: &BackendFunction,
    declaration: FunctionDeclaration,
    failure: &super::super::BackendFailure,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<(), BackendEmitError> {
    validate_failure_return(function, failure.layout)?;
    if failure.layout.niche_pointer {
        return emit_niche_failure(context, builder, function, declaration, failure, values);
    }
    let enum_type = super::values::llvm_type(context, function.return_value.ty)?;
    let temporary = allocate_failure_storage(builder, enum_type, failure.layout.layout)?;
    store_discriminant(context, builder, temporary, failure)?;
    store_payload(context, builder, temporary, failure, values)?;
    let value = load_failure(builder, enum_type, temporary)?;
    finish_failure(builder, declaration, value);
    Ok(())
}

fn emit_niche_failure(
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    function: &BackendFunction,
    declaration: FunctionDeclaration,
    failure: &super::super::BackendFailure,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<(), BackendEmitError> {
    if declaration.return_slot {
        return Err(BackendEmitError(
            "LLVM niche-pointer failure cannot use an aggregate return slot".to_owned(),
        ));
    }
    let value = match failure.payload {
        Some(payload_id) => values.get(&payload_id).copied().ok_or_else(|| {
            BackendEmitError(format!("LLVM niche failure payload {} is unavailable", payload_id.0))
        })?,
        None => unsafe {
            LLVMConstNull(super::values::llvm_type(context, function.return_value.ty)?)
        },
    };
    if unsafe { LLVMGetTypeKind(LLVMTypeOf(value)) } != LLVMPointerTypeKind {
        return Err(BackendEmitError(
            "LLVM niche-pointer failure payload must be a pointer".to_owned(),
        ));
    }
    unsafe { llvm_sys::core::LLVMBuildRet(builder, value) };
    Ok(())
}

fn validate_failure_return(
    function: &BackendFunction,
    layout: BackendEnumLayout,
) -> Result<(), BackendEmitError> {
    let return_layout = match function.return_value.ty {
        BackendValueType::Aggregate { layout }
        | BackendValueType::Region { layout }
        | BackendValueType::Enum { layout: super::super::BackendEnumLayout { layout, .. } }
        | BackendValueType::NichePointerOption { layout } => layout,
        _ => {
            return Err(BackendEmitError(
                "LLVM typed failure requires an aggregate enum return type".to_owned(),
            ));
        }
    };
    if return_layout != layout.layout {
        return Err(BackendEmitError(
            "LLVM typed failure enum layout does not match the function return layout".to_owned(),
        ));
    }
    Ok(())
}

fn allocate_failure_storage(
    builder: LLVMBuilderRef,
    enum_type: llvm_sys::prelude::LLVMTypeRef,
    layout: BackendLayout,
) -> Result<LLVMValueRef, BackendEmitError> {
    let temporary = unsafe { LLVMBuildAlloca(builder, enum_type, b"failure\0".as_ptr().cast()) };
    if temporary.is_null() {
        return Err(BackendEmitError("LLVM failed to allocate typed failure value".to_owned()));
    }
    unsafe { LLVMSetAlignment(temporary, layout.alignment) };
    let zero = unsafe { LLVMConstNull(enum_type) };
    if unsafe { LLVMBuildStore(builder, zero, temporary) }.is_null() {
        return Err(BackendEmitError("LLVM failed to initialize typed failure value".to_owned()));
    }
    Ok(temporary)
}

fn store_discriminant(
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    temporary: LLVMValueRef,
    failure: &super::super::BackendFailure,
) -> Result<(), BackendEmitError> {
    let byte_type = unsafe { LLVMInt8TypeInContext(context) };
    let ty = discriminant_type(context, failure.layout)?;
    let value = unsafe { LLVMConstInt(ty, u64::from(failure.discriminant), 0) };
    let address = byte_offset(
        builder,
        context,
        temporary,
        byte_type,
        failure.layout.discriminant_offset,
        "failure.discriminant",
    )?;
    if unsafe { LLVMBuildStore(builder, value, address) }.is_null() {
        return Err(BackendEmitError("LLVM failed to store enum discriminant".to_owned()));
    }
    Ok(())
}

fn store_payload(
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    temporary: LLVMValueRef,
    failure: &super::super::BackendFailure,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<(), BackendEmitError> {
    let (Some(payload_id), Some(payload_layout)) = (failure.payload, failure.layout.payload) else {
        return Ok(());
    };
    let payload = values.get(&payload_id).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM failure payload {} is unavailable", payload_id.0))
    })?;
    validate_integer_payload(payload, payload_layout)?;
    let byte_type = unsafe { LLVMInt8TypeInContext(context) };
    let address = byte_offset(
        builder,
        context,
        temporary,
        byte_type,
        failure.layout.payload_offset,
        "failure.payload",
    )?;
    if unsafe { LLVMBuildStore(builder, payload, address) }.is_null() {
        return Err(BackendEmitError("LLVM failed to store enum payload".to_owned()));
    }
    Ok(())
}

fn load_failure(
    builder: LLVMBuilderRef,
    enum_type: llvm_sys::prelude::LLVMTypeRef,
    temporary: LLVMValueRef,
) -> Result<LLVMValueRef, BackendEmitError> {
    let value = unsafe {
        LLVMBuildLoad2(builder, enum_type, temporary, b"failure.value\0".as_ptr().cast())
    };
    (!value.is_null())
        .then_some(value)
        .ok_or_else(|| BackendEmitError("LLVM failed to load typed failure value".to_owned()))
}

fn finish_failure(builder: LLVMBuilderRef, declaration: FunctionDeclaration, value: LLVMValueRef) {
    if declaration.return_slot {
        let slot = unsafe { LLVMGetParam(declaration.value, 0) };
        unsafe {
            LLVMBuildStore(builder, value, slot);
            LLVMBuildRetVoid(builder);
        }
    } else {
        unsafe { llvm_sys::core::LLVMBuildRet(builder, value) };
    }
}

fn discriminant_type(
    context: llvm_sys::prelude::LLVMContextRef,
    layout: BackendEnumLayout,
) -> Result<llvm_sys::prelude::LLVMTypeRef, BackendEmitError> {
    Ok(match layout.discriminant_size {
        1 => unsafe { LLVMInt8TypeInContext(context) },
        2 => unsafe { LLVMInt16TypeInContext(context) },
        4 => unsafe { LLVMInt32TypeInContext(context) },
        8 => unsafe { LLVMInt64TypeInContext(context) },
        _ => return Err(BackendEmitError("LLVM enum discriminant width is invalid".to_owned())),
    })
}

fn byte_offset(
    builder: LLVMBuilderRef,
    context: llvm_sys::prelude::LLVMContextRef,
    pointer: LLVMValueRef,
    byte_type: llvm_sys::prelude::LLVMTypeRef,
    offset: u32,
    name: &str,
) -> Result<LLVMValueRef, BackendEmitError> {
    let name = std::ffi::CString::new(name)
        .map_err(|_| BackendEmitError("LLVM failure address name contains NUL".to_owned()))?;
    let mut index = unsafe { LLVMConstInt(LLVMInt64TypeInContext(context), u64::from(offset), 0) };
    let address =
        unsafe { LLVMBuildGEP2(builder, byte_type, pointer, &mut index, 1, name.as_ptr()) };
    (!address.is_null())
        .then_some(address)
        .ok_or_else(|| BackendEmitError("LLVM failed to address typed failure field".to_owned()))
}

fn validate_integer_payload(
    payload: LLVMValueRef,
    layout: super::super::BackendLayout,
) -> Result<(), BackendEmitError> {
    if unsafe { LLVMGetTypeKind(LLVMTypeOf(payload)) } != LLVMIntegerTypeKind {
        return Err(BackendEmitError(
            "LLVM typed failure currently requires an integer payload".to_owned(),
        ));
    }
    let width = unsafe { LLVMGetIntTypeWidth(LLVMTypeOf(payload)) };
    let bytes = u32::from(width.div_ceil(8));
    if bytes > layout.size {
        return Err(BackendEmitError(
            "LLVM typed failure payload exceeds its declared layout".to_owned(),
        ));
    }
    Ok(())
}
