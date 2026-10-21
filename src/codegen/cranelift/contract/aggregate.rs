use std::collections::BTreeMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::Module;
use cranelift_object::ObjectModule;

use super::super::aggregate::Declaration;
use super::super::native::BackendEmitError;
use super::{arrays, packed};
use crate::codegen::{BackendAggregateField, BackendInstruction, BackendValueId, BackendValueType};

pub(super) fn lower_aggregate_construct(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    declarations: &BTreeMap<String, Declaration>,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::AggregateConstruct { result, layout, fields, .. } = instruction else {
        return Err(BackendEmitError("Cranelift aggregate construction expected".to_owned()));
    };
    let address = allocate_aggregate(module, builder, *layout, declarations)?;
    zero_initialize_aggregate(module, builder, address, *layout);
    for field in fields {
        store_aggregate_field(module, builder, address, field, values)?;
    }
    values.insert(*result, address);
    Ok(())
}

fn allocate_aggregate(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    layout: crate::codegen::BackendLayout,
    declarations: &BTreeMap<String, Declaration>,
) -> Result<cranelift_codegen::ir::Value, BackendEmitError> {
    if layout.size >= crate::codegen::LARGE_AGGREGATE_THRESHOLD {
        let declaration =
            declarations.get(crate::runtime::AGGREGATE_ALLOCATE_SYMBOL).ok_or_else(|| {
                BackendEmitError("large aggregate allocator is unavailable".to_owned())
            })?;
        let reference = module.declare_func_in_func(declaration.id, builder.func);
        let size = builder.ins().iconst(module.isa().pointer_type(), i64::from(layout.size));
        let alignment =
            builder.ins().iconst(module.isa().pointer_type(), i64::from(layout.alignment));
        let call = builder.ins().call(reference, &[size, alignment]);
        Ok(builder.inst_results(call)[0])
    } else {
        let alignment = u8::try_from(layout.alignment)
            .map_err(|_| BackendEmitError("Cranelift aggregate alignment exceeds u8".to_owned()))?;
        let slot = builder.func.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
            cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
            layout.size,
            alignment.trailing_zeros() as u8,
        ));
        Ok(builder.ins().stack_addr(module.isa().pointer_type(), slot, 0))
    }
}

fn store_aggregate_field(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    address: cranelift_codegen::ir::Value,
    field: &BackendAggregateField,
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let value = values.get(&field.value).copied().ok_or_else(|| {
        BackendEmitError(format!(
            "Cranelift aggregate field value {} is unavailable",
            field.value.0
        ))
    })?;
    let destination = builder.ins().iadd_imm_s(address, i64::from(field.offset));
    if matches!(
        field.ty,
        BackendValueType::Aggregate { .. }
            | BackendValueType::Region { .. }
            | BackendValueType::Enum { .. }
    ) {
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
        let ty = super::declarations::scalar_type(module, field.ty)?.ok_or_else(|| {
            BackendEmitError("Cranelift aggregate field must have a storable value type".to_owned())
        })?;
        if builder.func.dfg.value_type(value) != ty {
            return Err(BackendEmitError(
                "Cranelift aggregate field value type mismatches layout".to_owned(),
            ));
        }
        builder.ins().store(MemFlagsData::new(), value, destination, 0);
    }
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
    let offset = i32::try_from(*offset)
        .map_err(|_| BackendEmitError("Cranelift aggregate field offset exceeds i32".to_owned()))?;
    if let BackendValueType::Aggregate { layout }
    | BackendValueType::Region { layout }
    | BackendValueType::Enum { layout: crate::codegen::BackendEnumLayout { layout, .. } } = ty
    {
        let source = builder.ins().iadd_imm_s(base, i64::from(offset));
        let _ = (module, layout);
        values.insert(*result, source);
    } else {
        let value_type = super::declarations::scalar_type(module, *ty)?.ok_or_else(|| {
            BackendEmitError("Cranelift aggregate field must have a scalar value type".to_owned())
        })?;
        values.insert(*result, builder.ins().load(value_type, MemFlagsData::new(), base, offset));
    }
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
    let offset = i32::try_from(*offset)
        .map_err(|_| BackendEmitError("Cranelift aggregate field offset exceeds i32".to_owned()))?;
    if let BackendValueType::Aggregate { layout }
    | BackendValueType::Region { layout }
    | BackendValueType::Enum { layout: crate::codegen::BackendEnumLayout { layout, .. } } = ty
    {
        let destination = builder.ins().iadd_imm_s(base, i64::from(offset));
        let alignment = u8::try_from(layout.alignment).map_err(|_| {
            BackendEmitError("Cranelift aggregate field alignment exceeds u8".to_owned())
        })?;
        builder.emit_small_memory_copy(
            module.isa().frontend_config(),
            destination,
            value,
            u64::from(layout.size),
            alignment,
            alignment,
            false,
            MemFlagsData::new(),
        );
    } else {
        let value_type = super::declarations::scalar_type(module, *ty)?.ok_or_else(|| {
            BackendEmitError("Cranelift aggregate field must have a scalar value type".to_owned())
        })?;
        if builder.func.dfg.value_type(value) != value_type {
            return Err(BackendEmitError(
                "Cranelift aggregate field store type mismatch".to_owned(),
            ));
        }
        builder.ins().store(MemFlagsData::new(), value, base, offset);
    }
    Ok(())
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

pub(super) fn lower_aggregate_instruction(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    declarations: &BTreeMap<String, Declaration>,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    match instruction {
        BackendInstruction::AggregateConstruct { .. } => {
            lower_aggregate_construct(module, builder, instruction, declarations, values)
        }
        BackendInstruction::AggregateFieldLoad { .. }
        | BackendInstruction::AggregateFieldStore { .. } => {
            lower_aggregate_field_instruction(module, builder, instruction, values)
        }
        BackendInstruction::PackedFieldLoad { .. } => {
            packed::lower_packed_field_load(module, builder, instruction, values)
        }
        BackendInstruction::PackedFieldStore { .. } => {
            packed::lower_packed_field_store(module, builder, instruction, values)
        }
        BackendInstruction::ArrayElementLoad { .. } => {
            arrays::lower_array_element_load(module, builder, instruction, values)
        }
        BackendInstruction::ArrayElementStore { .. } => {
            arrays::lower_array_element_store(module, builder, instruction, values)
        }
        _ => Err(BackendEmitError("Cranelift aggregate instruction expected".to_owned())),
    }
}
