use std::collections::BTreeMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::Module;
use cranelift_object::ObjectModule;

use super::super::native::BackendEmitError;
use crate::codegen::{BackendInstruction, BackendValueId, BackendValueType};

pub(super) fn lower_array_element_load(
    module: &mut ObjectModule,
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
        big_endian,
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
    if matches!(element_type, BackendValueType::Aggregate { .. } | BackendValueType::Region { .. })
    {
        values.insert(*result, address);
        return Ok(());
    }
    let value_type = super::scalar_type(module, *element_type)?
        .ok_or_else(|| BackendEmitError("Cranelift array element must be scalar".to_owned()))?;
    let mut value = builder.ins().load(value_type, MemFlagsData::new(), address, 0);
    if *big_endian && value_type.bits() > 8 {
        value = builder.ins().bswap(value);
    }
    values.insert(*result, value);
    Ok(())
}

pub(super) fn lower_array_element_store(
    module: &mut ObjectModule,
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
        big_endian,
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
    let mut value = values.get(value).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift array value {} is unavailable", value.0))
    })?;
    let pointer_type = module.isa().pointer_type();
    let index = coerce_array_index(builder, index, pointer_type);
    let offset = builder.ins().imul_imm_u(index, i64::from(element_layout.size));
    let address = builder.ins().iadd(base, offset);
    if matches!(element_type, BackendValueType::Aggregate { .. } | BackendValueType::Region { .. })
    {
        store_array_aggregate(module, builder, address, value, *element_layout, pointer_type)?;
        return Ok(());
    }
    let value_type = super::scalar_type(module, *element_type)?
        .ok_or_else(|| BackendEmitError("Cranelift array element must be scalar".to_owned()))?;
    if builder.func.dfg.value_type(value) != value_type {
        return Err(BackendEmitError("Cranelift array element store type mismatch".to_owned()));
    }
    if *big_endian && value_type.bits() > 8 {
        value = builder.ins().bswap(value);
    }
    builder.ins().store(MemFlagsData::new(), value, address, 0);
    Ok(())
}

fn store_array_aggregate(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    address: cranelift_codegen::ir::Value,
    value: cranelift_codegen::ir::Value,
    layout: crate::codegen::BackendLayout,
    pointer_type: cranelift_codegen::ir::Type,
) -> Result<(), BackendEmitError> {
    if builder.func.dfg.value_type(value) != pointer_type {
        return Err(BackendEmitError(
            "Cranelift aggregate array value must be a pointer".to_owned(),
        ));
    }
    let alignment = u8::try_from(layout.alignment).map_err(|_| {
        BackendEmitError("Cranelift aggregate array alignment exceeds u8".to_owned())
    })?;
    builder.emit_small_memory_copy(
        module.isa().frontend_config(),
        address,
        value,
        u64::from(layout.size),
        alignment,
        alignment,
        false,
        MemFlagsData::new(),
    );
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
