use std::collections::BTreeMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData, Type};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::Module;
use cranelift_object::ObjectModule;

use super::super::native::BackendEmitError;
use crate::codegen::{BackendFunction, BackendInstruction, BackendValueId};

pub(super) fn lower_return(
    module: &ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    function: &BackendFunction,
    return_slot: Option<cranelift_codegen::ir::Value>,
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
    aggregate_values: &BTreeMap<BackendValueId, Vec<cranelift_codegen::ir::Value>>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::Return { value, .. } = instruction else {
        return Err(BackendEmitError("Cranelift return instruction expected".to_owned()));
    };
    if let crate::codegen::BackendReturnMode::ReturnSlot { layout } = function.return_value.passing
    {
        return lower_return_slot(module, builder, *value, function, return_slot, layout, values);
    }
    if let Some(value_id) = value
        && matches!(
            function.return_value.passing,
            crate::codegen::BackendReturnMode::DirectAggregate { .. }
        )
    {
        return lower_direct_aggregate_return(builder, *value_id, aggregate_values);
    }
    if function.hosted_entry_return && value.is_none() {
        let result = builder.ins().iconst(cranelift_codegen::ir::types::I32, 0);
        builder.ins().return_(&[result]);
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

fn lower_return_slot(
    module: &ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    value: Option<BackendValueId>,
    function: &BackendFunction,
    return_slot: Option<cranelift_codegen::ir::Value>,
    layout: crate::codegen::BackendLayout,
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let value_id = value.ok_or_else(|| {
        BackendEmitError("Cranelift return-slot instruction has no value".to_owned())
    })?;
    let source = values.get(&value_id).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift return-slot value {} is unavailable", value_id.0))
    })?;
    let destination = return_slot
        .ok_or_else(|| BackendEmitError("Cranelift return slot is unavailable".to_owned()))?;
    if builder.func.dfg.value_type(source) != module.isa().pointer_type() {
        return Err(BackendEmitError(format!(
            "Cranelift return-slot value in `{}` must be an aggregate pointer",
            function.symbol.as_str()
        )));
    }
    if let crate::codegen::BackendValueType::NichePointerOption { payload_layout, .. } =
        function.return_value.ty
    {
        return lower_niche_return_slot(module, builder, source, destination, payload_layout);
    }
    let alignment = u8::try_from(layout.alignment)
        .map_err(|_| BackendEmitError("Cranelift return-slot alignment exceeds u8".to_owned()))?;
    builder.emit_small_memory_copy(
        module.isa().frontend_config(),
        destination,
        source,
        u64::from(layout.size),
        alignment,
        alignment,
        false,
        MemFlagsData::new(),
    );
    builder.ins().return_(&[]);
    Ok(())
}

fn lower_niche_return_slot(
    module: &ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    source: cranelift_codegen::ir::Value,
    destination: cranelift_codegen::ir::Value,
    payload_layout: Option<crate::codegen::BackendLayout>,
) -> Result<(), BackendEmitError> {
    if let Some(payload_layout) = payload_layout {
        return lower_borrowed_view_return(module, builder, source, destination, payload_layout);
    }
    if source != destination {
        builder.ins().store(MemFlagsData::new(), source, destination, 0);
    }
    builder.ins().return_(&[]);
    Ok(())
}

fn lower_borrowed_view_return(
    module: &ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    source: cranelift_codegen::ir::Value,
    destination: cranelift_codegen::ir::Value,
    payload_layout: crate::codegen::BackendLayout,
) -> Result<(), BackendEmitError> {
    let null_block = builder.create_block();
    let some_block = builder.create_block();
    let null = builder.ins().iconst(module.isa().pointer_type(), 0);
    let is_null = builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, source, null);
    builder.ins().brif(is_null, null_block, &[], some_block, &[]);
    builder.switch_to_block(null_block);
    builder.ins().return_(&[null]);
    builder.switch_to_block(some_block);
    let alignment = u8::try_from(payload_layout.alignment)
        .map_err(|_| BackendEmitError("Cranelift borrowed view alignment exceeds u8".to_owned()))?;
    builder.emit_small_memory_copy(
        module.isa().frontend_config(),
        destination,
        source,
        u64::from(payload_layout.size),
        alignment,
        alignment,
        false,
        MemFlagsData::new(),
    );
    builder.ins().return_(&[destination]);
    Ok(())
}

fn lower_direct_aggregate_return(
    builder: &mut FunctionBuilder<'_>,
    value: BackendValueId,
    aggregate_values: &BTreeMap<BackendValueId, Vec<cranelift_codegen::ir::Value>>,
) -> Result<(), BackendEmitError> {
    let fragments = aggregate_values.get(&value).ok_or_else(|| {
        BackendEmitError(format!("Cranelift aggregate return value {} is unavailable", value.0))
    })?;
    builder.ins().return_(fragments);
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
