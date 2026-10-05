use cranelift_codegen::ir::{InstBuilder, Value, types};
use cranelift_frontend::FunctionBuilder;

use super::super::layout::{LayoutRegistry, PackFieldLayout};
use super::super::native::NativeEmitError;
use super::super::types::NativeType;
use super::packs::{field_mask, mapped_bit_offset_for_storage, sign_extend_field};

pub(super) fn lower_indexed_pack_field_address(
    function: &mut FunctionBuilder<'_>,
    storage: Value,
    storage_type: NativeType,
    field: &PackFieldLayout,
) -> Result<Value, NativeEmitError> {
    if !matches!(storage_type, NativeType::Array(_)) {
        return Err(NativeEmitError(
            "indexed packed fields require byte-addressable storage".to_owned(),
        ));
    }
    if field.offset % 8 != 0 || field.width % 8 != 0 {
        return Err(NativeEmitError(
            "indexed packed fields require byte-aligned elements".to_owned(),
        ));
    }
    Ok(function.ins().iadd_imm_s(storage, i64::from(u32::from(field.offset / 8))))
}

pub(super) fn lower_inline_pack_field(
    function: &mut FunctionBuilder<'_>,
    storage: Value,
    field: &PackFieldLayout,
    storage_type: NativeType,
    endianness: crate::ast::LayoutEndianness,
    layouts: &LayoutRegistry,
) -> Result<Value, NativeEmitError> {
    let storage_size = layouts
        .type_size(storage_type)
        .ok_or_else(|| NativeEmitError("missing inline packed storage size".to_owned()))?;
    let mapped_offset = mapped_bit_offset_for_storage(field, endianness, storage_size)?;
    let bit_offset = mapped_offset % 8;
    let load_type = inline_container_type(bit_offset, field.width)?;
    let address = function.ins().iadd_imm_s(storage, i64::from(u32::from(mapped_offset / 8)));
    let mut value =
        function.ins().load(load_type, cranelift_codegen::ir::MemFlagsData::new(), address, 0);
    if matches!(endianness, crate::ast::LayoutEndianness::Big) && load_type.bits() > 8 {
        value = function.ins().bswap(value);
    }
    if bit_offset != 0 {
        let shift = function.ins().iconst(load_type, i64::from(bit_offset));
        value = function.ins().ushr(value, shift);
    }
    let mask = field_mask(function, field.width, load_type.bits() as u16, load_type);
    let value = function.ins().band(value, mask);
    let value = sign_extend_field(
        function,
        value,
        field.ty,
        field.width,
        load_type.bits() as u16,
        load_type,
    );
    Ok(super::super::expressions::coerce_to_ir_type(function, value, layouts.ir_type(field.ty)?))
}

pub(super) fn lower_inline_pack_field_write(
    function: &mut FunctionBuilder<'_>,
    storage: Value,
    new_value: Value,
    field: &PackFieldLayout,
    storage_type: NativeType,
    endianness: crate::ast::LayoutEndianness,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    let storage_size = layouts
        .type_size(storage_type)
        .ok_or_else(|| NativeEmitError("missing inline packed storage size".to_owned()))?;
    let mapped_offset = mapped_bit_offset_for_storage(field, endianness, storage_size)?;
    let bit_offset = mapped_offset % 8;
    let container_type = inline_container_type(bit_offset, field.width)?;
    let storage_address =
        function.ins().iadd_imm_s(storage, i64::from(u32::from(mapped_offset / 8)));
    let current = function.ins().load(
        container_type,
        cranelift_codegen::ir::MemFlagsData::new(),
        storage_address,
        0,
    );
    let mask = field_mask(function, field.width, container_type.bits() as u16, container_type);
    let shift = function.ins().iconst(container_type, i64::from(bit_offset));
    let shifted_mask = function.ins().ishl(mask, shift);
    let inverse_mask = function.ins().bnot(shifted_mask);
    let cleared = function.ins().band(current, inverse_mask);
    let value = super::super::expressions::coerce_to_ir_type(function, new_value, container_type);
    let masked_value = function.ins().band(value, mask);
    let value = function.ins().ishl(masked_value, shift);
    let mut updated = function.ins().bor(cleared, value);
    if matches!(endianness, crate::ast::LayoutEndianness::Big) && container_type.bits() > 8 {
        updated = function.ins().bswap(updated);
    }
    function.ins().store(cranelift_codegen::ir::MemFlagsData::new(), updated, storage_address, 0);
    Ok(())
}

fn inline_container_type(
    bit_offset: u16,
    field_width: u8,
) -> Result<cranelift_codegen::ir::Type, NativeEmitError> {
    let required_bits = bit_offset
        .checked_add(u16::from(field_width))
        .ok_or_else(|| NativeEmitError("inline packed field width overflow".to_owned()))?;
    match required_bits {
        0..=8 => Ok(types::I8),
        9..=16 => Ok(types::I16),
        17..=32 => Ok(types::I32),
        33..=64 => Ok(types::I64),
        65..=128 => Ok(types::I128),
        _ => {
            Err(NativeEmitError("inline packed fields exceed the native integer width".to_owned()))
        }
    }
}
