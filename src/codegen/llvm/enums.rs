use std::collections::BTreeMap;

use llvm_sys::core::{
    LLVMBuildAlloca, LLVMBuildGEP2, LLVMBuildLoad2, LLVMBuildMemMove, LLVMBuildStore, LLVMConstInt,
    LLVMConstNull, LLVMInt8TypeInContext, LLVMSetAlignment,
};
use llvm_sys::prelude::{LLVMBuilderRef, LLVMContextRef, LLVMTypeRef, LLVMValueRef};

use super::super::{
    BackendEmitError, BackendEnumLayout, BackendInstruction, BackendValueId, BackendValueType,
};

pub(super) fn emit_enum_discriminant(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let BackendInstruction::EnumDiscriminant { result, subject, layout, .. } = instruction else {
        return Err(BackendEmitError("LLVM enum discriminant instruction expected".to_owned()));
    };
    let temporary = enum_temporary(context, builder, *subject, *layout, values)?;
    let address = enum_field_address(context, builder, temporary, layout.discriminant_offset)?;
    let discriminant_type = integer_type(context, layout.discriminant_size)?;
    let value = unsafe {
        LLVMBuildLoad2(builder, discriminant_type, address, b"discriminant\0".as_ptr().cast())
    };
    if value.is_null() {
        return Err(BackendEmitError("LLVM failed to load enum discriminant".to_owned()));
    }
    values.insert(*result, value);
    Ok(false)
}

pub(super) fn emit_enum_payload(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let (result, subject, layout, payload_type) = match instruction {
        BackendInstruction::EnumPayload { result, subject, layout, payload_type, .. }
        | BackendInstruction::EnumPayloadBinding {
            result, subject, layout, payload_type, ..
        } => (result, subject, layout, payload_type),
        _ => return Err(BackendEmitError("LLVM enum payload instruction expected".to_owned())),
    };
    let temporary = enum_temporary(context, builder, *subject, *layout, values)?;
    let address = enum_field_address(context, builder, temporary, layout.payload_offset)?;
    let llvm_type = super::values::llvm_type(context, *payload_type)?;
    let value = if matches!(
        payload_type,
        super::super::BackendValueType::Aggregate { .. }
            | super::super::BackendValueType::Enum { .. }
    ) {
        address
    } else {
        unsafe { LLVMBuildLoad2(builder, llvm_type, address, b"payload\0".as_ptr().cast()) }
    };
    if value.is_null() {
        return Err(BackendEmitError("LLVM failed to load enum payload".to_owned()));
    }
    values.insert(*result, value);
    Ok(false)
}

pub(super) fn emit_enum_construct(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let BackendInstruction::EnumConstruct {
        result,
        discriminant,
        payload,
        layout,
        payload_type,
        ..
    } = instruction
    else {
        return Err(BackendEmitError("LLVM enum constructor instruction expected".to_owned()));
    };
    let (enum_type, temporary) =
        initialize_enum_constructor(context, builder, layout, *discriminant)?;
    let payload_value = values.get(payload).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM enum payload {} is unavailable", payload.0))
    })?;
    let payload_address = enum_field_address(context, builder, temporary, layout.payload_offset)?;
    let _ = super::values::llvm_type(context, *payload_type)?;
    let Some(payload_layout) = layout.payload else {
        return Err(BackendEmitError("LLVM enum constructor payload layout is missing".to_owned()));
    };
    if matches!(payload_type, BackendValueType::Aggregate { .. } | BackendValueType::Enum { .. }) {
        emit_aggregate_payload_copy(
            context,
            builder,
            payload_address,
            payload_value,
            payload_layout,
        )?;
    } else if unsafe { LLVMBuildStore(builder, payload_value, payload_address) }.is_null() {
        return Err(BackendEmitError("LLVM failed to store enum constructor payload".to_owned()));
    }
    let value = unsafe {
        LLVMBuildLoad2(builder, enum_type, temporary, b"enum.construct.value\0".as_ptr().cast())
    };
    if value.is_null() {
        return Err(BackendEmitError("LLVM failed to load enum constructor value".to_owned()));
    }
    values.insert(*result, value);
    Ok(false)
}

fn initialize_enum_constructor(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    layout: &BackendEnumLayout,
    discriminant: u32,
) -> Result<(LLVMTypeRef, LLVMValueRef), BackendEmitError> {
    let enum_type = super::values::llvm_type(
        context,
        super::super::BackendValueType::Aggregate { layout: layout.layout },
    )?;
    let temporary =
        unsafe { LLVMBuildAlloca(builder, enum_type, b"enum.construct\0".as_ptr().cast()) };
    if temporary.is_null() {
        return Err(BackendEmitError("LLVM failed to allocate enum constructor value".to_owned()));
    }
    unsafe { LLVMSetAlignment(temporary, layout.layout.alignment) };
    if unsafe { LLVMBuildStore(builder, LLVMConstNull(enum_type), temporary) }.is_null() {
        return Err(BackendEmitError(
            "LLVM failed to initialize enum constructor value".to_owned(),
        ));
    }
    let address = enum_field_address(context, builder, temporary, layout.discriminant_offset)?;
    let value = unsafe {
        LLVMConstInt(integer_type(context, layout.discriminant_size)?, u64::from(discriminant), 0)
    };
    if unsafe { LLVMBuildStore(builder, value, address) }.is_null() {
        return Err(BackendEmitError(
            "LLVM failed to store enum constructor discriminant".to_owned(),
        ));
    }
    Ok((enum_type, temporary))
}

fn emit_aggregate_payload_copy(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    destination: LLVMValueRef,
    source: LLVMValueRef,
    layout: super::super::BackendLayout,
) -> Result<(), BackendEmitError> {
    let size = unsafe {
        LLVMConstInt(llvm_sys::core::LLVMInt64TypeInContext(context), u64::from(layout.size), 0)
    };
    let copied = unsafe {
        LLVMBuildMemMove(builder, destination, layout.alignment, source, layout.alignment, size)
    };
    if copied.is_null() {
        return Err(BackendEmitError("LLVM failed to copy enum constructor payload".to_owned()));
    }
    Ok(())
}

fn enum_temporary(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    subject: BackendValueId,
    layout: BackendEnumLayout,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<LLVMValueRef, BackendEmitError> {
    let subject = values
        .get(&subject)
        .copied()
        .ok_or_else(|| BackendEmitError(format!("LLVM enum value {} is unavailable", subject.0)))?;
    let enum_type = super::values::llvm_type(
        context,
        super::super::BackendValueType::Aggregate { layout: layout.layout },
    )?;
    let temporary = unsafe { LLVMBuildAlloca(builder, enum_type, b"enum.value\0".as_ptr().cast()) };
    if temporary.is_null() {
        return Err(BackendEmitError("LLVM failed to allocate enum temporary".to_owned()));
    }
    if unsafe { LLVMBuildStore(builder, subject, temporary) }.is_null() {
        return Err(BackendEmitError("LLVM failed to store enum temporary".to_owned()));
    }
    Ok(temporary)
}

fn enum_field_address(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    temporary: LLVMValueRef,
    offset: u32,
) -> Result<LLVMValueRef, BackendEmitError> {
    let index = unsafe {
        LLVMConstInt(llvm_sys::core::LLVMInt64TypeInContext(context), u64::from(offset), 0)
    };
    let address = unsafe {
        LLVMBuildGEP2(
            builder,
            LLVMInt8TypeInContext(context),
            temporary,
            &index as *const _ as *mut _,
            1,
            b"enum.field\0".as_ptr().cast(),
        )
    };
    (!address.is_null())
        .then_some(address)
        .ok_or_else(|| BackendEmitError("LLVM failed to address enum field".to_owned()))
}

fn integer_type(
    context: LLVMContextRef,
    width: u8,
) -> Result<llvm_sys::prelude::LLVMTypeRef, BackendEmitError> {
    match width {
        1 => Ok(unsafe { llvm_sys::core::LLVMInt8TypeInContext(context) }),
        2 => Ok(unsafe { llvm_sys::core::LLVMInt16TypeInContext(context) }),
        4 => Ok(unsafe { llvm_sys::core::LLVMInt32TypeInContext(context) }),
        8 => Ok(unsafe { llvm_sys::core::LLVMInt64TypeInContext(context) }),
        _ => Err(BackendEmitError("LLVM enum discriminant width is invalid".to_owned())),
    }
}
