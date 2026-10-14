use std::collections::BTreeMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData, Type, types};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::Module;
use cranelift_object::ObjectModule;

use super::super::native::BackendEmitError;
use crate::codegen::{BackendInstruction, BackendValueId, BackendValueType};

pub(super) fn lower(
    module: &ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    return_slot: Option<cranelift_codegen::ir::Value>,
    return_value_ids: &std::collections::BTreeSet<BackendValueId>,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    match instruction {
        BackendInstruction::EnumDiscriminant { .. } => {
            lower_discriminant(module, builder, instruction, values)
        }
        BackendInstruction::EnumPayload { .. } | BackendInstruction::EnumPayloadBinding { .. } => {
            lower_payload(module, builder, instruction, values)
        }
        BackendInstruction::EnumConstruct { .. } => lower_construct(
            module,
            builder,
            instruction,
            return_slot.filter(|_| {
                matches!(
                    instruction,
                    BackendInstruction::EnumConstruct { result, .. }
                        if return_value_ids.contains(result)
                )
            }),
            values,
        ),
        _ => Err(BackendEmitError("Cranelift enum instruction expected".to_owned())),
    }
}

fn lower_construct(
    module: &ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    return_slot: Option<cranelift_codegen::ir::Value>,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::EnumConstruct { discriminant, payload, layout, payload_type, .. } =
        instruction
    else {
        return Err(BackendEmitError("Cranelift enum constructor instruction expected".to_owned()));
    };
    if layout.niche_pointer {
        let value = match payload {
            Some(payload) => values.get(payload).copied().ok_or_else(|| {
                BackendEmitError(format!(
                    "Cranelift niche Option payload {} is unavailable",
                    payload.0
                ))
            })?,
            None => builder.ins().iconst(module.isa().pointer_type(), 0),
        };
        let _ = discriminant;
        let _ = payload_type;
        let _ = return_slot;
        values.insert(
            match instruction {
                BackendInstruction::EnumConstruct { result, .. } => *result,
                _ => unreachable!(),
            },
            value,
        );
        return Ok(());
    }
    let alignment = u8::try_from(layout.layout.alignment)
        .map_err(|_| BackendEmitError("Cranelift enum alignment exceeds u8".to_owned()))?;
    let slot = if let Some(return_slot) = return_slot {
        return_slot
    } else {
        let stack_slot =
            builder.func.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
                cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                layout.layout.size,
                alignment.trailing_zeros() as u8,
            ));
        builder.ins().stack_addr(module.isa().pointer_type(), stack_slot, 0)
    };
    let discriminant_type = integer_type_for(layout.discriminant_size)?;
    let discriminant_value = builder.ins().iconst(discriminant_type, i64::from(*discriminant));
    builder.ins().store(
        MemFlagsData::new(),
        discriminant_value,
        slot,
        layout.discriminant_offset as i32,
    );
    lower_construct_payload(module, builder, slot, *layout, payload, payload_type, values)?;
    if let BackendInstruction::EnumConstruct { result, .. } = instruction {
        values.insert(*result, slot);
    }
    Ok(())
}

fn lower_construct_payload(
    module: &ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    slot: cranelift_codegen::ir::Value,
    layout: super::super::super::BackendEnumLayout,
    payload: &Option<BackendValueId>,
    payload_type: &Option<BackendValueType>,
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let (Some(payload), Some(payload_type), Some(payload_layout)) =
        (payload, payload_type, layout.payload)
    else {
        return Ok(());
    };
    let payload_value = values.get(payload).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift enum payload {} is unavailable", payload.0))
    })?;
    let payload_address = builder.ins().iadd_imm_s(slot, i64::from(layout.payload_offset));
    if is_scalar_payload(*payload_type) {
        builder.ins().store(MemFlagsData::new(), payload_value, slot, layout.payload_offset as i32);
        return Ok(());
    }
    if builder.func.dfg.value_type(payload_value) != module.isa().pointer_type() {
        return Err(BackendEmitError(
            "Cranelift aggregate enum payload must be a pointer".to_owned(),
        ));
    }
    let alignment = u8::try_from(payload_layout.alignment)
        .map_err(|_| BackendEmitError("Cranelift enum payload alignment is invalid".to_owned()))?;
    builder.emit_small_memory_copy(
        module.isa().frontend_config(),
        payload_address,
        payload_value,
        u64::from(payload_layout.size),
        alignment,
        alignment,
        false,
        MemFlagsData::new(),
    );
    Ok(())
}

pub(super) fn lower_discriminant(
    module: &ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::EnumDiscriminant { result, subject, layout, .. } = instruction else {
        return Err(BackendEmitError(
            "Cranelift enum discriminant instruction expected".to_owned(),
        ));
    };
    if layout.niche_pointer {
        let subject = values.get(subject).copied().ok_or_else(|| {
            BackendEmitError(format!("Cranelift niche Option {} is unavailable", subject.0))
        })?;
        let comparison =
            builder.ins().icmp_imm_s(cranelift_codegen::ir::condcodes::IntCC::Equal, subject, 0);
        let one = builder.ins().iconst(types::I32, 1);
        let zero = builder.ins().iconst(types::I32, 0);
        let value = builder.ins().select(comparison, zero, one);
        values.insert(*result, value);
        return Ok(());
    }
    let subject = enum_storage_pointer(builder, module.isa().pointer_type(), *subject, values)?;
    let ty = integer_type_for(layout.discriminant_size)?;
    let value =
        builder.ins().load(ty, MemFlagsData::new(), subject, layout.discriminant_offset as i32);
    values.insert(*result, value);
    Ok(())
}

pub(super) fn lower_payload(
    module: &ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let (result, subject, layout, payload_type) = match instruction {
        BackendInstruction::EnumPayload { result, subject, layout, payload_type, .. }
        | BackendInstruction::EnumPayloadBinding {
            result, subject, layout, payload_type, ..
        } => (result, subject, layout, payload_type),
        _ => {
            return Err(BackendEmitError("Cranelift enum payload instruction expected".to_owned()));
        }
    };
    let subject = enum_storage_pointer(builder, module.isa().pointer_type(), *subject, values)?;
    let payload = match payload_type {
        BackendValueType::Bool | BackendValueType::Integer { .. } => {
            let ty = scalar_type(*payload_type)?;
            builder.ins().load(ty, MemFlagsData::new(), subject, layout.payload_offset as i32)
        }
        BackendValueType::Pointer | BackendValueType::NichePointerOption { .. } => subject,
        BackendValueType::Region { .. } => builder.ins().load(
            module.isa().pointer_type(),
            MemFlagsData::new(),
            subject,
            layout.payload_offset as i32,
        ),
        BackendValueType::Aggregate { .. } | BackendValueType::Enum { .. } => {
            builder.ins().iadd_imm_s(subject, i64::from(layout.payload_offset))
        }
        _ => {
            return Err(BackendEmitError(
                "Cranelift enum payload type is outside the supported contract".to_owned(),
            ));
        }
    };
    values.insert(*result, payload);
    Ok(())
}

fn enum_storage_pointer(
    builder: &mut FunctionBuilder<'_>,
    pointer_type: Type,
    subject: BackendValueId,
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<cranelift_codegen::ir::Value, BackendEmitError> {
    let subject = values.get(&subject).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift enum value {} is unavailable", subject.0))
    })?;
    if builder.func.dfg.value_type(subject) != pointer_type {
        return Err(BackendEmitError("Cranelift enum storage is not a pointer".to_owned()));
    }
    Ok(subject)
}

fn scalar_type(ty: BackendValueType) -> Result<Type, BackendEmitError> {
    match ty {
        BackendValueType::Bool | BackendValueType::Integer { width: 8, .. } => Ok(types::I8),
        BackendValueType::Integer { width: 16, .. } => Ok(types::I16),
        BackendValueType::Integer { width: 32, .. } => Ok(types::I32),
        BackendValueType::Integer { width: 64, .. } => Ok(types::I64),
        _ => Err(BackendEmitError("Cranelift enum payload is not scalar".to_owned())),
    }
}

fn is_scalar_payload(ty: BackendValueType) -> bool {
    matches!(
        ty,
        BackendValueType::Bool | BackendValueType::Integer { .. } | BackendValueType::Region { .. }
    )
}

pub(super) fn integer_type_for(size: u8) -> Result<Type, BackendEmitError> {
    match size {
        1 => Ok(types::I8),
        2 => Ok(types::I16),
        4 => Ok(types::I32),
        8 => Ok(types::I64),
        _ => Err(BackendEmitError("Cranelift enum discriminant width is invalid".to_owned())),
    }
}
