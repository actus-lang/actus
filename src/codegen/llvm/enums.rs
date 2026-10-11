use std::collections::BTreeMap;

use llvm_sys::core::{
    LLVMBuildAlloca, LLVMBuildGEP2, LLVMBuildLoad2, LLVMBuildStore, LLVMConstInt, LLVMConstNull,
    LLVMInt8TypeInContext, LLVMSetAlignment,
};
use llvm_sys::prelude::{LLVMBuilderRef, LLVMContextRef, LLVMValueRef};

use super::super::{BackendEmitError, BackendEnumLayout, BackendInstruction, BackendValueId};

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
    let BackendInstruction::EnumPayload { result, subject, layout, payload_type, .. } = instruction
    else {
        return Err(BackendEmitError("LLVM enum payload instruction expected".to_owned()));
    };
    let temporary = enum_temporary(context, builder, *subject, *layout, values)?;
    let address = enum_field_address(context, builder, temporary, layout.payload_offset)?;
    let llvm_type = super::values::llvm_type(context, *payload_type)?;
    let value =
        unsafe { LLVMBuildLoad2(builder, llvm_type, address, b"payload\0".as_ptr().cast()) };
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
    let discriminant_address =
        enum_field_address(context, builder, temporary, layout.discriminant_offset)?;
    let discriminant_value = unsafe {
        LLVMConstInt(integer_type(context, layout.discriminant_size)?, u64::from(*discriminant), 0)
    };
    if unsafe { LLVMBuildStore(builder, discriminant_value, discriminant_address) }.is_null() {
        return Err(BackendEmitError(
            "LLVM failed to store enum constructor discriminant".to_owned(),
        ));
    }
    let payload_value = values.get(payload).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM enum payload {} is unavailable", payload.0))
    })?;
    let payload_address = enum_field_address(context, builder, temporary, layout.payload_offset)?;
    let _ = super::values::llvm_type(context, *payload_type)?;
    if unsafe { LLVMBuildStore(builder, payload_value, payload_address) }.is_null() {
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
