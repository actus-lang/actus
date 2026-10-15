use std::collections::BTreeMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData};
use cranelift_frontend::FunctionBuilder;
use cranelift_object::ObjectModule;

use super::super::native::BackendEmitError;
use crate::codegen::{BackendInstruction, BackendValueId, BackendValueType};

pub(super) fn lower_packed_field_load(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
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
        return Err(BackendEmitError("Cranelift packed field load expected".to_owned()));
    };
    let base = values.get(base).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift packed field base {} is unavailable", base.0))
    })?;
    let (storage_type, byte_offset, local_bit_offset) =
        packed_access(*storage_layout, *bit_offset, *width, *big_endian)?;
    let storage_base = builder.ins().iadd_imm_s(base, i64::from(byte_offset));
    let mut storage = builder.ins().load(storage_type, MemFlagsData::new(), storage_base, 0);
    if *big_endian && storage_type.bits() > 8 {
        storage = builder.ins().bswap(storage);
    }
    let shifted = if local_bit_offset == 0 {
        storage
    } else {
        let shift = builder.ins().iconst(storage_type, i64::from(local_bit_offset));
        builder.ins().ushr(storage, shift)
    };
    let mask = packed_mask(builder, builder.func.dfg.value_type(shifted), *width)?;
    let extracted = builder.ins().band(shifted, mask);
    let target_type = super::declarations::scalar_type(module, *ty)?
        .ok_or_else(|| BackendEmitError("Cranelift packed field must be scalar".to_owned()))?;
    let extracted = coerce_integer(builder, extracted, target_type);
    let extracted = sign_extend_packed_value(builder, extracted, target_type, *width, *ty);
    values.insert(*result, extracted);
    Ok(())
}

pub(super) fn lower_packed_field_store(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
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
        return Err(BackendEmitError("Cranelift packed field store expected".to_owned()));
    };
    let base = values.get(base).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift packed field base {} is unavailable", base.0))
    })?;
    let value = values.get(value).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift packed field value {} is unavailable", value.0))
    })?;
    let (storage_type, byte_offset, local_bit_offset) =
        packed_access(*storage_layout, *bit_offset, *width, *big_endian)?;
    let storage_base = builder.ins().iadd_imm_s(base, i64::from(byte_offset));
    let mut current = builder.ins().load(storage_type, MemFlagsData::new(), storage_base, 0);
    if *big_endian && storage_type.bits() > 8 {
        current = builder.ins().bswap(current);
    }
    let current_type = builder.func.dfg.value_type(current);
    let mask = packed_mask(builder, current_type, *width)?;
    let shift = builder.ins().iconst(current_type, i64::from(local_bit_offset));
    let shifted_mask = if local_bit_offset == 0 { mask } else { builder.ins().ishl(mask, shift) };
    let inverse = builder.ins().bnot(shifted_mask);
    let cleared = builder.ins().band(current, inverse);
    let _ = module;
    let value = coerce_integer(builder, value, current_type);
    let value = builder.ins().band(value, mask);
    let value = if local_bit_offset == 0 { value } else { builder.ins().ishl(value, shift) };
    let mut updated = builder.ins().bor(cleared, value);
    if *big_endian && storage_type.bits() > 8 {
        updated = builder.ins().bswap(updated);
    }
    builder.ins().store(MemFlagsData::new(), updated, storage_base, 0);
    let _ = ty;
    Ok(())
}

fn packed_access(
    layout: crate::codegen::BackendLayout,
    bit_offset: u16,
    width: u8,
    big_endian: bool,
) -> Result<(cranelift_codegen::ir::Type, u32, u8), BackendEmitError> {
    let total_bits = layout.size.checked_mul(8).ok_or_else(|| {
        BackendEmitError("Cranelift packed storage layout size overflows bit width".to_owned())
    })?;
    let end = u32::from(bit_offset).checked_add(u32::from(width)).ok_or_else(|| {
        BackendEmitError("Cranelift packed field bit range overflows layout".to_owned())
    })?;
    if end > total_bits {
        return Err(BackendEmitError("Cranelift packed field exceeds storage layout".to_owned()));
    }
    if big_endian && layout.size > 1 {
        let (storage_type, _, _) = packed_storage_type(layout, 0, total_bits as u8)?;
        return Ok((
            storage_type,
            0,
            u8::try_from(bit_offset).map_err(|_| {
                BackendEmitError(
                    "Cranelift packed field bit offset exceeds scalar width".to_owned(),
                )
            })?,
        ));
    }
    packed_storage_type(layout, u32::from(bit_offset), width)
}

fn packed_storage_type(
    layout: crate::codegen::BackendLayout,
    bit_offset: u32,
    width: u8,
) -> Result<(cranelift_codegen::ir::Type, u32, u8), BackendEmitError> {
    let total_bits = layout.size.checked_mul(8).ok_or_else(|| {
        BackendEmitError("Cranelift packed storage layout size overflows bit width".to_owned())
    })?;
    let end = bit_offset.checked_add(u32::from(width)).ok_or_else(|| {
        BackendEmitError("Cranelift packed field bit range overflows layout".to_owned())
    })?;
    if end > total_bits {
        return Err(BackendEmitError("Cranelift packed field exceeds storage layout".to_owned()));
    }
    let byte_offset = bit_offset / 8;
    let local_bit_offset = (bit_offset % 8) as u8;
    let required_bytes = (u32::from(local_bit_offset) + u32::from(width)).div_ceil(8);
    let storage_type = match required_bytes {
        1 => cranelift_codegen::ir::types::I8,
        2 => cranelift_codegen::ir::types::I16,
        3 | 4 => cranelift_codegen::ir::types::I32,
        5..=8 => cranelift_codegen::ir::types::I64,
        _ => {
            return Err(BackendEmitError(
                "Cranelift packed field exceeds the supported 8-byte span".to_owned(),
            ));
        }
    };
    Ok((storage_type, byte_offset, local_bit_offset))
}

fn packed_mask(
    builder: &mut FunctionBuilder<'_>,
    ty: cranelift_codegen::ir::Type,
    width: u8,
) -> Result<cranelift_codegen::ir::Value, BackendEmitError> {
    let value = if width == 64 { -1 } else { (1i64 << width) - 1 };
    Ok(builder.ins().iconst(ty, value))
}

fn coerce_integer(
    builder: &mut FunctionBuilder<'_>,
    value: cranelift_codegen::ir::Value,
    target: cranelift_codegen::ir::Type,
) -> cranelift_codegen::ir::Value {
    let source = builder.func.dfg.value_type(value);
    if source == target {
        value
    } else if source.bits() < target.bits() {
        builder.ins().uextend(target, value)
    } else {
        builder.ins().ireduce(target, value)
    }
}

fn sign_extend_packed_value(
    builder: &mut FunctionBuilder<'_>,
    value: cranelift_codegen::ir::Value,
    target: cranelift_codegen::ir::Type,
    width: u8,
    ty: BackendValueType,
) -> cranelift_codegen::ir::Value {
    let signed = matches!(ty, BackendValueType::Integer { signed: true, .. });
    if !signed || u32::from(width) >= target.bits() {
        return value;
    }
    let shift_amount = target.bits() - u32::from(width);
    let shift = builder.ins().iconst(target, i64::from(shift_amount));
    let shifted = builder.ins().ishl(value, shift);
    builder.ins().sshr(shifted, shift)
}
