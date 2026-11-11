use std::collections::BTreeMap;

use llvm_sys::core::{
    LLVMBuildLShr, LLVMBuildLoad2, LLVMBuildOr, LLVMBuildShl, LLVMBuildStore, LLVMBuildTrunc,
    LLVMBuildZExt, LLVMConstInt, LLVMInt8TypeInContext, LLVMInt64TypeInContext,
};
use llvm_sys::prelude::{LLVMBuilderRef, LLVMContextRef, LLVMValueRef};

use super::super::super::{BackendEmitError, BackendValueId};
use super::{PackedField, cast_integer, field_address, integer_width};

pub(super) fn emit_wide_packed_field_load(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    base: LLVMValueRef,
    field: PackedField,
    result: BackendValueId,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let byte_offset = byte_field_offset(
        field.storage_layout.size,
        field.bit_offset,
        field.width,
        field.big_endian,
    )?;
    let byte_count = u32::from(field.width / 8);
    let mut combined = unsafe { LLVMConstInt(LLVMInt64TypeInContext(context), 0, 0) };
    for index in 0..byte_count {
        let byte = load_wide_byte(context, builder, base, byte_offset + index)?;
        let shift = if field.big_endian { (byte_count - index - 1) * 8 } else { index * 8 };
        let shifted = if shift == 0 {
            byte
        } else {
            unsafe {
                LLVMBuildShl(
                    builder,
                    byte,
                    LLVMConstInt(LLVMInt64TypeInContext(context), u64::from(shift), 0),
                    c"packed.wide.shift".as_ptr().cast(),
                )
            }
        };
        combined =
            unsafe { LLVMBuildOr(builder, combined, shifted, c"packed.wide.or".as_ptr().cast()) };
    }
    let lowered = cast_integer(
        builder,
        combined,
        64,
        super::super::values::llvm_type(context, field.ty)?,
        integer_width(field.ty)?,
        "packed.wide.result",
    )?;
    values.insert(result, lowered);
    Ok(false)
}

fn load_wide_byte(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    base: LLVMValueRef,
    offset: u32,
) -> Result<LLVMValueRef, BackendEmitError> {
    let address = field_address(context, builder, base, offset)?;
    let byte = unsafe {
        LLVMBuildLoad2(
            builder,
            LLVMInt8TypeInContext(context),
            address,
            c"packed.wide.load".as_ptr().cast(),
        )
    };
    if byte.is_null() {
        return Err(BackendEmitError("LLVM failed to load wide packed byte".to_owned()));
    }
    let byte = unsafe {
        LLVMBuildZExt(
            builder,
            byte,
            LLVMInt64TypeInContext(context),
            c"packed.wide.byte".as_ptr().cast(),
        )
    };
    (!byte.is_null())
        .then_some(byte)
        .ok_or_else(|| BackendEmitError("LLVM failed to widen wide packed byte".to_owned()))
}

pub(super) fn emit_wide_packed_field_store(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    base: LLVMValueRef,
    field: PackedField,
    value: BackendValueId,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let byte_offset = byte_field_offset(
        field.storage_layout.size,
        field.bit_offset,
        field.width,
        field.big_endian,
    )?;
    let byte_count = u32::from(field.width / 8);
    let value = values
        .get(&value)
        .copied()
        .ok_or_else(|| BackendEmitError("LLVM wide packed value is unavailable".to_owned()))?;
    let value = cast_integer(
        builder,
        value,
        integer_width(field.ty)?,
        unsafe { LLVMInt64TypeInContext(context) },
        64,
        "packed.wide.store.value",
    )?;
    for index in 0..byte_count {
        let shift = if field.big_endian { (byte_count - index - 1) * 8 } else { index * 8 };
        let shifted = if shift == 0 {
            value
        } else {
            unsafe {
                LLVMBuildLShr(
                    builder,
                    value,
                    LLVMConstInt(LLVMInt64TypeInContext(context), u64::from(shift), 0),
                    c"packed.wide.store.shift".as_ptr().cast(),
                )
            }
        };
        let byte = unsafe {
            LLVMBuildTrunc(
                builder,
                shifted,
                LLVMInt8TypeInContext(context),
                c"packed.wide.store.byte".as_ptr().cast(),
            )
        };
        let address = field_address(context, builder, base, byte_offset + index)?;
        if unsafe { LLVMBuildStore(builder, byte, address) }.is_null() {
            return Err(BackendEmitError("LLVM failed to store wide packed byte".to_owned()));
        }
    }
    Ok(false)
}

fn byte_field_offset(
    storage_size: u32,
    bit_offset: u16,
    width: u8,
    big_endian: bool,
) -> Result<u32, BackendEmitError> {
    if !bit_offset.is_multiple_of(8) || width == 0 || !width.is_multiple_of(8) {
        return Err(BackendEmitError(
            "LLVM wide packed fields must be byte-aligned and byte-sized".to_owned(),
        ));
    }
    let start = u32::from(bit_offset) / 8;
    let count = u32::from(width) / 8;
    if start.checked_add(count).is_none_or(|end| end > storage_size) {
        return Err(BackendEmitError("LLVM wide packed field exceeds storage".to_owned()));
    }
    Ok(if big_endian { storage_size - start - count } else { start })
}
