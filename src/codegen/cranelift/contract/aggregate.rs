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
