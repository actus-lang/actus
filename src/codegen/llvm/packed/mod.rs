mod wide;

use std::collections::BTreeMap;

use llvm_sys::core::{
    LLVMBuildAShr, LLVMBuildAnd, LLVMBuildGEP2, LLVMBuildLShr, LLVMBuildLoad2, LLVMBuildNot,
    LLVMBuildOr, LLVMBuildShl, LLVMBuildStore, LLVMBuildTrunc, LLVMBuildZExt, LLVMConstInt,
    LLVMInt8TypeInContext, LLVMInt16TypeInContext, LLVMInt32TypeInContext, LLVMInt64TypeInContext,
};
use llvm_sys::prelude::{LLVMBuilderRef, LLVMContextRef, LLVMTypeRef, LLVMValueRef};

use super::super::{BackendEmitError, BackendInstruction, BackendValueId, BackendValueType};

#[derive(Clone, Copy)]
pub(super) struct PackedField {
    pub(super) storage_layout: super::super::BackendLayout,
    pub(super) bit_offset: u16,
    pub(super) width: u8,
    pub(super) big_endian: bool,
    pub(super) ty: BackendValueType,
}

pub(super) fn emit_packed_field_load(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let BackendInstruction::PackedFieldLoad {
        result,
        base,
        storage_layout,
        bit_offset,
        width,
        big_endian,
        ty,
        ..
    } = instruction
    else {
        return Err(BackendEmitError("LLVM packed field load expected".to_owned()));
    };
    let base = values.get(base).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM packed field base {} is unavailable", base.0))
    })?;
    if storage_layout.size > 8 {
        return wide::emit_wide_packed_field_load(
            context,
            builder,
            base,
            PackedField {
                storage_layout: *storage_layout,
                bit_offset: *bit_offset,
                width: *width,
                big_endian: *big_endian,
                ty: *ty,
            },
            *result,
            values,
        );
    }
    emit_scalar_packed_field_load(
        context,
        builder,
        base,
        PackedField {
            storage_layout: *storage_layout,
            bit_offset: *bit_offset,
            width: *width,
            big_endian: *big_endian,
            ty: *ty,
        },
        *result,
        values,
    )
}

pub(super) fn emit_packed_field_store(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    instruction: &BackendInstruction,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let BackendInstruction::PackedFieldStore {
        base,
        storage_layout,
        bit_offset,
        width,
        big_endian,
        value,
        ty,
        ..
    } = instruction
    else {
        return Err(BackendEmitError("LLVM packed field store expected".to_owned()));
    };
    let base = values.get(base).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM packed field base {} is unavailable", base.0))
    })?;
    if storage_layout.size > 8 {
        return wide::emit_wide_packed_field_store(
            context,
            builder,
            base,
            PackedField {
                storage_layout: *storage_layout,
                bit_offset: *bit_offset,
                width: *width,
                big_endian: *big_endian,
                ty: *ty,
            },
            *value,
            values,
        );
    }
    emit_scalar_packed_field_store(
        context,
        builder,
        base,
        PackedField {
            storage_layout: *storage_layout,
            bit_offset: *bit_offset,
            width: *width,
            big_endian: *big_endian,
            ty: *ty,
        },
        *value,
        values,
    )
}

fn emit_scalar_packed_field_load(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    base: LLVMValueRef,
    field: PackedField,
    result: BackendValueId,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let storage_type = packed_storage_type(context, field.storage_layout)?;
    let address = field_address(context, builder, base, 0)?;
    let mut storage = unsafe {
        LLVMBuildLoad2(builder, storage_type, address, c"packed.field.load".as_ptr().cast())
    };
    if storage.is_null() {
        return Err(BackendEmitError("LLVM failed to load packed storage".to_owned()));
    }
    let storage_bits = field
        .storage_layout
        .size
        .checked_mul(8)
        .ok_or_else(|| BackendEmitError("LLVM packed storage bit width overflow".to_owned()))?;
    if field.big_endian && storage_bits > 8 {
        storage = byte_swap(context, builder, storage, storage_bits as u8);
    }
    let logical_field = PackedField { big_endian: false, ..field };
    let extracted =
        extract_packed_value(builder, storage, storage_type, storage_bits, &logical_field)?;
    let lowered_result = cast_integer(
        builder,
        extracted,
        storage_bits,
        super::values::llvm_type(context, field.ty)?,
        integer_width(field.ty)?,
        "packed.field.result",
    )?;
    let lowered_result = sign_extend_packed_value(
        builder,
        lowered_result,
        integer_width(field.ty)?,
        field.width,
        field.ty,
    );
    values.insert(result, lowered_result);
    Ok(false)
}

fn emit_scalar_packed_field_store(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    base: LLVMValueRef,
    field: PackedField,
    value_id: BackendValueId,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let value = values.get(&value_id).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM packed field value {} is unavailable", value_id.0))
    })?;
    let storage_type = packed_storage_type(context, field.storage_layout)?;
    let address = field_address(context, builder, base, 0)?;
    let current = unsafe {
        LLVMBuildLoad2(builder, storage_type, address, c"packed.field.current".as_ptr().cast())
    };
    if current.is_null() {
        return Err(BackendEmitError("LLVM failed to load packed storage for store".to_owned()));
    }
    let storage_bits = field
        .storage_layout
        .size
        .checked_mul(8)
        .ok_or_else(|| BackendEmitError("LLVM packed storage bit width overflow".to_owned()))?;
    let logical_current = if field.big_endian && storage_bits > 8 {
        byte_swap(context, builder, current, storage_bits as u8)
    } else {
        current
    };
    let logical_field = PackedField { big_endian: false, ..field };
    let mut updated = update_packed_value(
        builder,
        logical_current,
        value,
        integer_width(field.ty)?,
        storage_type,
        storage_bits,
        &logical_field,
    )?;
    if field.big_endian && storage_bits > 8 {
        updated = byte_swap(context, builder, updated, storage_bits as u8);
    }
    if unsafe { LLVMBuildStore(builder, updated, address) }.is_null() {
        return Err(BackendEmitError("LLVM failed to store packed storage".to_owned()));
    }
    Ok(false)
}

fn extract_packed_value(
    builder: LLVMBuilderRef,
    storage: LLVMValueRef,
    storage_type: LLVMTypeRef,
    storage_bits: u32,
    field: &PackedField,
) -> Result<LLVMValueRef, BackendEmitError> {
    let shift = packed_shift(storage_bits, field.bit_offset, field.width, field.big_endian)?;
    let shifted = if shift == 0 {
        storage
    } else {
        unsafe {
            LLVMBuildLShr(
                builder,
                storage,
                LLVMConstInt(storage_type, u64::from(shift), 0),
                c"packed.field.shift".as_ptr().cast(),
            )
        }
    };
    let mask = packed_mask(storage_type, field.width)?;
    Ok(unsafe { LLVMBuildAnd(builder, shifted, mask, c"packed.field.mask".as_ptr().cast()) })
}

fn sign_extend_packed_value(
    builder: LLVMBuilderRef,
    value: LLVMValueRef,
    target_width: u32,
    field_width: u8,
    ty: BackendValueType,
) -> LLVMValueRef {
    if !matches!(ty, BackendValueType::Integer { signed: true, .. })
        || u32::from(field_width) >= target_width
    {
        return value;
    }
    let value_type = unsafe { llvm_sys::core::LLVMTypeOf(value) };
    let shift =
        unsafe { LLVMConstInt(value_type, u64::from(target_width - u32::from(field_width)), 0) };
    let shifted =
        unsafe { LLVMBuildShl(builder, value, shift, c"packed.field.sign.shift".as_ptr().cast()) };
    unsafe { LLVMBuildAShr(builder, shifted, shift, c"packed.field.sign.extend".as_ptr().cast()) }
}

fn byte_swap(
    _context: LLVMContextRef,
    builder: LLVMBuilderRef,
    value: LLVMValueRef,
    width: u8,
) -> LLVMValueRef {
    let ty = unsafe { llvm_sys::core::LLVMTypeOf(value) };
    let bytes = u32::from(width / 8);
    let byte_mask = unsafe { LLVMConstInt(ty, 0xff, 0) };
    let mut result = unsafe { LLVMConstInt(ty, 0, 0) };
    for source in 0..bytes {
        let source_shift = unsafe { LLVMConstInt(ty, u64::from(source * 8), 0) };
        let shifted = if source == 0 {
            value
        } else {
            unsafe {
                LLVMBuildLShr(builder, value, source_shift, c"packed.bswap.read".as_ptr().cast())
            }
        };
        let byte = unsafe {
            LLVMBuildAnd(builder, shifted, byte_mask, c"packed.bswap.mask".as_ptr().cast())
        };
        let destination = bytes - source - 1;
        let destination_shift = unsafe { LLVMConstInt(ty, u64::from(destination * 8), 0) };
        let placed = if destination == 0 {
            byte
        } else {
            unsafe {
                LLVMBuildShl(
                    builder,
                    byte,
                    destination_shift,
                    c"packed.bswap.write".as_ptr().cast(),
                )
            }
        };
        result =
            unsafe { LLVMBuildOr(builder, result, placed, c"packed.bswap.or".as_ptr().cast()) };
    }
    result
}

fn update_packed_value(
    builder: LLVMBuilderRef,
    current: LLVMValueRef,
    value: LLVMValueRef,
    value_width: u32,
    storage_type: LLVMTypeRef,
    storage_bits: u32,
    field: &PackedField,
) -> Result<LLVMValueRef, BackendEmitError> {
    let shift = packed_shift(storage_bits, field.bit_offset, field.width, field.big_endian)?;
    let mask = packed_mask(storage_type, field.width)?;
    let shifted_mask = if shift == 0 {
        mask
    } else {
        unsafe {
            LLVMBuildShl(
                builder,
                mask,
                LLVMConstInt(storage_type, u64::from(shift), 0),
                c"packed.field.shifted.mask".as_ptr().cast(),
            )
        }
    };
    let inverse = unsafe {
        LLVMBuildNot(builder, shifted_mask, c"packed.field.inverse.mask".as_ptr().cast())
    };
    let cleared =
        unsafe { LLVMBuildAnd(builder, current, inverse, c"packed.field.cleared".as_ptr().cast()) };
    let value = cast_integer(
        builder,
        value,
        value_width,
        storage_type,
        storage_bits,
        "packed.field.value",
    )?;
    let value =
        unsafe { LLVMBuildAnd(builder, value, mask, c"packed.field.value.mask".as_ptr().cast()) };
    let value = if shift == 0 {
        value
    } else {
        unsafe {
            LLVMBuildShl(
                builder,
                value,
                LLVMConstInt(storage_type, u64::from(shift), 0),
                c"packed.field.value.shift".as_ptr().cast(),
            )
        }
    };
    Ok(unsafe { LLVMBuildOr(builder, cleared, value, c"packed.field.updated".as_ptr().cast()) })
}

fn packed_storage_type(
    context: LLVMContextRef,
    layout: super::super::BackendLayout,
) -> Result<LLVMTypeRef, BackendEmitError> {
    let ty = unsafe {
        match layout.size {
            1 => LLVMInt8TypeInContext(context),
            2 => LLVMInt16TypeInContext(context),
            4 => LLVMInt32TypeInContext(context),
            8 => LLVMInt64TypeInContext(context),
            _ => std::ptr::null_mut(),
        }
    };
    (!ty.is_null()).then_some(ty).ok_or_else(|| {
        BackendEmitError("LLVM packed storage must be 1, 2, 4, or 8 bytes".to_owned())
    })
}

pub(super) fn integer_width(ty: BackendValueType) -> Result<u32, BackendEmitError> {
    match ty {
        BackendValueType::Integer { width: 8, .. } => Ok(8),
        BackendValueType::Integer { width: 16, .. } => Ok(16),
        BackendValueType::Integer { width: 32, .. } => Ok(32),
        BackendValueType::Integer { width: 64, .. } => Ok(64),
        _ => Err(BackendEmitError("LLVM packed fields require integer values".to_owned())),
    }
}

fn packed_mask(ty: LLVMTypeRef, width: u8) -> Result<LLVMValueRef, BackendEmitError> {
    let value = if width == 64 { u64::MAX } else { (1u64 << width) - 1 };
    Ok(unsafe { LLVMConstInt(ty, value, 0) })
}

fn packed_shift(
    storage_bits: u32,
    bit_offset: u16,
    width: u8,
    big_endian: bool,
) -> Result<u32, BackendEmitError> {
    let end = u32::from(bit_offset) + u32::from(width);
    if width == 0 || end > storage_bits {
        return Err(BackendEmitError("LLVM packed field exceeds storage".to_owned()));
    }
    Ok(if big_endian { storage_bits - end } else { u32::from(bit_offset) })
}

pub(super) fn cast_integer(
    builder: LLVMBuilderRef,
    value: LLVMValueRef,
    source_width: u32,
    target_type: LLVMTypeRef,
    target_width: u32,
    name: &str,
) -> Result<LLVMValueRef, BackendEmitError> {
    if source_width == target_width {
        return Ok(value);
    }
    let value = unsafe {
        if source_width < target_width {
            LLVMBuildZExt(
                builder,
                value,
                target_type,
                std::ffi::CString::new(name).unwrap().as_ptr(),
            )
        } else {
            LLVMBuildTrunc(
                builder,
                value,
                target_type,
                std::ffi::CString::new(name).unwrap().as_ptr(),
            )
        }
    };
    (!value.is_null())
        .then_some(value)
        .ok_or_else(|| BackendEmitError("LLVM integer cast failed".to_owned()))
}

pub(super) fn field_address(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    base: LLVMValueRef,
    offset: u32,
) -> Result<LLVMValueRef, BackendEmitError> {
    let index = unsafe { LLVMConstInt(LLVMInt64TypeInContext(context), u64::from(offset), 0) };
    let address = unsafe {
        LLVMBuildGEP2(
            builder,
            LLVMInt8TypeInContext(context),
            base,
            &index as *const _ as *mut _,
            1,
            c"packed.field.address".as_ptr().cast(),
        )
    };
    (!address.is_null())
        .then_some(address)
        .ok_or_else(|| BackendEmitError("LLVM failed to address packed field".to_owned()))
}
