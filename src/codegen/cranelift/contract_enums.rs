use std::collections::BTreeMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData, Type, types};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::Module;
use cranelift_object::ObjectModule;

use super::native::BackendEmitError;
use crate::codegen::{BackendInstruction, BackendValueId, BackendValueType};

pub(super) fn lower(
    module: &ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    match instruction {
        BackendInstruction::EnumDiscriminant { .. } => {
            lower_discriminant(module, builder, instruction, values)
        }
        BackendInstruction::EnumPayload { .. } | BackendInstruction::EnumPayloadBinding { .. } => {
            lower_payload(module, builder, instruction, values)
        }
        BackendInstruction::EnumConstruct { .. } => Err(BackendEmitError(
            "Cranelift neutral enum construction is not implemented".to_owned(),
        )),
        _ => Err(BackendEmitError("Cranelift enum instruction expected".to_owned())),
    }
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
    let subject = enum_storage_pointer(builder, module.isa().pointer_type(), *subject, values)?;
    let ty = integer_type(layout.discriminant_size)?;
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
        BackendValueType::Aggregate { .. }
        | BackendValueType::Region { .. }
        | BackendValueType::Enum { .. } => {
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

fn integer_type(size: u8) -> Result<Type, BackendEmitError> {
    match size {
        1 => Ok(types::I8),
        2 => Ok(types::I16),
        4 => Ok(types::I32),
        8 => Ok(types::I64),
        _ => Err(BackendEmitError("Cranelift enum discriminant width is invalid".to_owned())),
    }
}
