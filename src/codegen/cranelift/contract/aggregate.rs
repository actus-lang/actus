use std::collections::BTreeMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::Module;
use cranelift_object::ObjectModule;

use super::super::native::BackendEmitError;
use crate::codegen::{BackendInstruction, BackendValueId, BackendValueType};

pub(super) fn lower_aggregate_construct(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::AggregateConstruct { result, layout, fields, .. } = instruction else {
        return Err(BackendEmitError("Cranelift aggregate construction expected".to_owned()));
    };
    let alignment = u8::try_from(layout.alignment)
        .map_err(|_| BackendEmitError("Cranelift aggregate alignment exceeds u8".to_owned()))?;
    let slot = builder.func.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
        cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
        layout.size,
        alignment.trailing_zeros() as u8,
    ));
    let address = builder.ins().stack_addr(module.isa().pointer_type(), slot, 0);
    zero_initialize_aggregate(module, builder, address, *layout);
    for field in fields {
        let value = values.get(&field.value).copied().ok_or_else(|| {
            BackendEmitError(format!(
                "Cranelift aggregate field value {} is unavailable",
                field.value.0
            ))
        })?;
        let destination = builder.ins().iadd_imm_s(address, i64::from(field.offset));
        if matches!(field.ty, BackendValueType::Aggregate { .. } | BackendValueType::Region { .. })
        {
            let alignment = u8::try_from(field.layout.alignment).map_err(|_| {
                BackendEmitError("Cranelift aggregate field alignment exceeds u8".to_owned())
            })?;
            builder.emit_small_memory_copy(
                module.isa().frontend_config(),
                destination,
                value,
                u64::from(field.layout.size),
                alignment,
                alignment,
                false,
                MemFlagsData::new(),
            );
        } else {
            let ty = super::scalar_type(module, field.ty)?.ok_or_else(|| {
                BackendEmitError(
                    "Cranelift aggregate field must have a storable value type".to_owned(),
                )
            })?;
            if builder.func.dfg.value_type(value) != ty {
                return Err(BackendEmitError(
                    "Cranelift aggregate field value type mismatches layout".to_owned(),
                ));
            }
            builder.ins().store(MemFlagsData::new(), value, destination, 0);
        }
    }
    values.insert(*result, address);
    Ok(())
}

fn zero_initialize_aggregate(
    module: &ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    address: cranelift_codegen::ir::Value,
    layout: crate::codegen::BackendLayout,
) {
    builder.emit_small_memset(
        module.isa().frontend_config(),
        address,
        0u8,
        u64::from(layout.size),
        layout.alignment.trailing_zeros() as u8,
        MemFlagsData::new(),
    );
}

pub(super) fn lower_aggregate_field_load(
    module: &ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::AggregateFieldLoad { result, base, offset, ty, .. } = instruction
    else {
        return Err(BackendEmitError("Cranelift aggregate field load expected".to_owned()));
    };
    let base = values.get(base).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift aggregate base {} is unavailable", base.0))
    })?;
    let value_type = super::scalar_type(module, *ty)?.ok_or_else(|| {
        BackendEmitError("Cranelift aggregate field must have a scalar value type".to_owned())
    })?;
    let offset = i32::try_from(*offset)
        .map_err(|_| BackendEmitError("Cranelift aggregate field offset exceeds i32".to_owned()))?;
    values.insert(*result, builder.ins().load(value_type, MemFlagsData::new(), base, offset));
    Ok(())
}

pub(super) fn lower_aggregate_field_store(
    module: &ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::AggregateFieldStore { base, offset, value, ty, .. } = instruction
    else {
        return Err(BackendEmitError("Cranelift aggregate field store expected".to_owned()));
    };
    let base = values.get(base).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift aggregate base {} is unavailable", base.0))
    })?;
    let value = values.get(value).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift aggregate field value {} is unavailable", value.0))
    })?;
    let value_type = super::scalar_type(module, *ty)?.ok_or_else(|| {
        BackendEmitError("Cranelift aggregate field must have a scalar value type".to_owned())
    })?;
    if builder.func.dfg.value_type(value) != value_type {
        return Err(BackendEmitError("Cranelift aggregate field store type mismatch".to_owned()));
    }
    let offset = i32::try_from(*offset)
        .map_err(|_| BackendEmitError("Cranelift aggregate field offset exceeds i32".to_owned()))?;
    builder.ins().store(MemFlagsData::new(), value, base, offset);
    Ok(())
}

pub(super) fn lower_array_element_load(
    module: &ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::ArrayElementLoad {
        result,
        base,
        index,
        element_layout,
        element_type,
        ..
    } = instruction
    else {
        return Err(BackendEmitError("Cranelift array element load expected".to_owned()));
    };
    let base = values.get(base).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift array base {} is unavailable", base.0))
    })?;
    let index = values.get(index).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift array index {} is unavailable", index.0))
    })?;
    let pointer_type = module.isa().pointer_type();
    let index = coerce_array_index(builder, index, pointer_type);
    let offset = builder.ins().imul_imm_u(index, i64::from(element_layout.size));
    let address = builder.ins().iadd(base, offset);
    let value_type = super::scalar_type(module, *element_type)?
        .ok_or_else(|| BackendEmitError("Cranelift array element must be scalar".to_owned()))?;
    values.insert(*result, builder.ins().load(value_type, MemFlagsData::new(), address, 0));
    Ok(())
}

pub(super) fn lower_array_element_store(
    module: &ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::ArrayElementStore {
        base,
        index,
        element_layout,
        element_type,
        value,
        ..
    } = instruction
    else {
        return Err(BackendEmitError("Cranelift array element store expected".to_owned()));
    };
    let base = values.get(base).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift array base {} is unavailable", base.0))
    })?;
    let index = values.get(index).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift array index {} is unavailable", index.0))
    })?;
    let value = values.get(value).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift array value {} is unavailable", value.0))
    })?;
    let pointer_type = module.isa().pointer_type();
    let index = coerce_array_index(builder, index, pointer_type);
    let offset = builder.ins().imul_imm_u(index, i64::from(element_layout.size));
    let address = builder.ins().iadd(base, offset);
    let value_type = super::scalar_type(module, *element_type)?
        .ok_or_else(|| BackendEmitError("Cranelift array element must be scalar".to_owned()))?;
    if builder.func.dfg.value_type(value) != value_type {
        return Err(BackendEmitError("Cranelift array element store type mismatch".to_owned()));
    }
    builder.ins().store(MemFlagsData::new(), value, address, 0);
    Ok(())
}

fn coerce_array_index(
    builder: &mut FunctionBuilder<'_>,
    index: cranelift_codegen::ir::Value,
    pointer_type: cranelift_codegen::ir::Type,
) -> cranelift_codegen::ir::Value {
    let source = builder.func.dfg.value_type(index);
    if source == pointer_type {
        index
    } else if source.bytes() < pointer_type.bytes() {
        builder.ins().uextend(pointer_type, index)
    } else {
        builder.ins().ireduce(pointer_type, index)
    }
}

pub(super) fn lower_aggregate_field_instruction(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    match instruction {
        BackendInstruction::AggregateFieldLoad { .. } => {
            lower_aggregate_field_load(module, builder, instruction, values)
        }
        BackendInstruction::AggregateFieldStore { .. } => {
            lower_aggregate_field_store(module, builder, instruction, values)
        }
        _ => Err(BackendEmitError("Cranelift aggregate field instruction expected".to_owned())),
    }
}

fn lower_packed_field_load(
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
    if *big_endian {
        return Err(BackendEmitError(
            "Cranelift neutral packed big-endian fields are not implemented".to_owned(),
        ));
    }
    let base = values.get(base).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift packed field base {} is unavailable", base.0))
    })?;
    let storage_type = packed_storage_type(*storage_layout)?;
    let storage = builder.ins().load(storage_type, MemFlagsData::new(), base, 0);
    let mask = packed_mask(builder, storage_type, *width)?;
    let shifted = if *bit_offset == 0 {
        storage
    } else {
        let shift = builder.ins().iconst(storage_type, i64::from(*bit_offset));
        builder.ins().ushr(storage, shift)
    };
    let extracted = builder.ins().band(shifted, mask);
    let target_type = super::scalar_type(module, *ty)?
        .ok_or_else(|| BackendEmitError("Cranelift packed field must be scalar".to_owned()))?;
    values.insert(*result, coerce_integer(builder, extracted, target_type));
    Ok(())
}

fn lower_packed_field_store(
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
    if *big_endian {
        return Err(BackendEmitError(
            "Cranelift neutral packed big-endian fields are not implemented".to_owned(),
        ));
    }
    let base = values.get(base).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift packed field base {} is unavailable", base.0))
    })?;
    let value = values.get(value).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift packed field value {} is unavailable", value.0))
    })?;
    let storage_type = packed_storage_type(*storage_layout)?;
    let current = builder.ins().load(storage_type, MemFlagsData::new(), base, 0);
    let mask = packed_mask(builder, storage_type, *width)?;
    let shift = builder.ins().iconst(storage_type, i64::from(*bit_offset));
    let shifted_mask = if *bit_offset == 0 { mask } else { builder.ins().ishl(mask, shift) };
    let inverse = builder.ins().bnot(shifted_mask);
    let cleared = builder.ins().band(current, inverse);
    let _ = module;
    let value = coerce_integer(builder, value, storage_type);
    let value = builder.ins().band(value, mask);
    let value = if *bit_offset == 0 { value } else { builder.ins().ishl(value, shift) };
    let updated = builder.ins().bor(cleared, value);
    builder.ins().store(MemFlagsData::new(), updated, base, 0);
    let _ = ty;
    Ok(())
}

fn packed_storage_type(
    layout: crate::codegen::BackendLayout,
) -> Result<cranelift_codegen::ir::Type, BackendEmitError> {
    match layout.size {
        1 => Ok(cranelift_codegen::ir::types::I8),
        2 => Ok(cranelift_codegen::ir::types::I16),
        4 => Ok(cranelift_codegen::ir::types::I32),
        8 => Ok(cranelift_codegen::ir::types::I64),
        _ => {
            Err(BackendEmitError("Cranelift packed storage must be 1, 2, 4, or 8 bytes".to_owned()))
        }
    }
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

pub(super) fn lower_aggregate_instruction(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    match instruction {
        BackendInstruction::AggregateConstruct { .. } => {
            lower_aggregate_construct(module, builder, instruction, values)
        }
        BackendInstruction::AggregateFieldLoad { .. }
        | BackendInstruction::AggregateFieldStore { .. } => {
            lower_aggregate_field_instruction(module, builder, instruction, values)
        }
        BackendInstruction::PackedFieldLoad { .. } => {
            lower_packed_field_load(module, builder, instruction, values)
        }
        BackendInstruction::PackedFieldStore { .. } => {
            lower_packed_field_store(module, builder, instruction, values)
        }
        BackendInstruction::ArrayElementLoad { .. } => {
            lower_array_element_load(module, builder, instruction, values)
        }
        BackendInstruction::ArrayElementStore { .. } => {
            lower_array_element_store(module, builder, instruction, values)
        }
        _ => Err(BackendEmitError("Cranelift aggregate instruction expected".to_owned())),
    }
}
