use std::collections::BTreeMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData, Type};
use cranelift_frontend::FunctionBuilder;

use super::super::native::BackendEmitError;
use crate::codegen::{BackendFunction, BackendInstruction, BackendValueId};

pub(super) fn lower_return(
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    function: &BackendFunction,
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
    aggregate_values: &BTreeMap<BackendValueId, Vec<cranelift_codegen::ir::Value>>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::Return { value, .. } = instruction else {
        return Err(BackendEmitError("Cranelift return instruction expected".to_owned()));
    };
    if matches!(function.return_value.passing, crate::codegen::BackendReturnMode::ReturnSlot { .. })
    {
        builder.ins().return_(&[]);
        return Ok(());
    }
    if let Some(value_id) = value
        && matches!(
            function.return_value.passing,
            crate::codegen::BackendReturnMode::DirectAggregate { .. }
        )
    {
        let fragments = aggregate_values.get(value_id).ok_or_else(|| {
            BackendEmitError(format!(
                "Cranelift aggregate return value {} is unavailable",
                value_id.0
            ))
        })?;
        builder.ins().return_(fragments);
        return Ok(());
    }
    let value = value
        .map(|value| {
            values.get(&value).copied().ok_or_else(|| {
                BackendEmitError(format!("Cranelift return value {} is unavailable", value.0))
            })
        })
        .transpose()?;
    match value {
        Some(value) => builder.ins().return_(&[value]),
        None => builder.ins().return_(&[]),
    };
    Ok(())
}

pub(super) fn lower_failure(
    pointer_type: Type,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    return_slot: Option<cranelift_codegen::ir::Value>,
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::Fail { failure, .. } = instruction else {
        return Err(BackendEmitError("Cranelift failure instruction expected".to_owned()));
    };
    if failure.layout.niche_pointer {
        return lower_niche_failure(pointer_type, builder, failure, return_slot, values);
    }
    let slot = return_slot.ok_or_else(|| {
        BackendEmitError("Cranelift typed failure requires a return slot".to_owned())
    })?;
    let discriminant_type = super::enums::integer_type_for(failure.layout.discriminant_size)?;
    let discriminant = builder.ins().iconst(discriminant_type, i64::from(failure.discriminant));
    builder.ins().store(
        MemFlagsData::new(),
        discriminant,
        slot,
        failure.layout.discriminant_offset as i32,
    );
    if let (Some(payload_id), Some(payload_layout)) = (failure.payload, failure.layout.payload) {
        let payload = values.get(&payload_id).copied().ok_or_else(|| {
            BackendEmitError(format!("Cranelift failure payload {} is unavailable", payload_id.0))
        })?;
        if !builder.func.dfg.value_type(payload).is_int()
            || builder.func.dfg.value_type(payload).bytes() > payload_layout.size
        {
            return Err(BackendEmitError(
                "Cranelift typed failure currently requires a fitting integer payload".to_owned(),
            ));
        }
        builder.ins().store(
            MemFlagsData::new(),
            payload,
            slot,
            failure.layout.payload_offset as i32,
        );
    }
    builder.ins().return_(&[]);
    Ok(())
}

fn lower_niche_failure(
    pointer_type: Type,
    builder: &mut FunctionBuilder<'_>,
    failure: &crate::codegen::BackendFailure,
    return_slot: Option<cranelift_codegen::ir::Value>,
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    if return_slot.is_some() {
        return Err(BackendEmitError(
            "Cranelift niche-pointer failure cannot use an aggregate return slot".to_owned(),
        ));
    }
    let value = failure
        .payload
        .and_then(|payload_id| values.get(&payload_id).copied())
        .unwrap_or_else(|| builder.ins().iconst(pointer_type, 0));
    if builder.func.dfg.value_type(value) != pointer_type {
        return Err(BackendEmitError(
            "Cranelift niche-pointer failure payload must be a pointer".to_owned(),
        ));
    }
    builder.ins().return_(&[value]);
    Ok(())
}
