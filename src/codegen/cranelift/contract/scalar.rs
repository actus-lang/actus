use std::collections::BTreeMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::Module;
use cranelift_object::ObjectModule;

use super::super::integer_ops::{
    lower_trap_if_array_index_out_of_bounds, lower_trap_if_shift_count_out_of_range,
    lower_trap_if_signed_division_overflow, lower_trap_if_zero,
};
use super::super::native::BackendEmitError;
use crate::codegen::{BackendInstruction, BackendValueId};

pub(super) fn lower_guard(
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    match instruction {
        BackendInstruction::TrapIfZero { .. } => lower_trap_if_zero(builder, instruction, values),
        BackendInstruction::TrapIfSignedDivisionOverflow { .. } => {
            lower_trap_if_signed_division_overflow(builder, instruction, values)
        }
        BackendInstruction::TrapIfShiftCountOutOfRange { .. } => {
            lower_trap_if_shift_count_out_of_range(builder, instruction, values)
        }
        BackendInstruction::TrapIfArrayIndexOutOfBounds { .. } => {
            lower_trap_if_array_index_out_of_bounds(builder, instruction, values)
        }
        _ => Err(BackendEmitError("Cranelift guard instruction is not supported".to_owned())),
    }
}

pub(super) fn lower_constant(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::Constant { result, ty, value, .. } = instruction else {
        return Err(BackendEmitError("Cranelift constant instruction is not supported".to_owned()));
    };
    let ty = super::scalar_type(module, *ty)?
        .ok_or_else(|| BackendEmitError("Cranelift constant cannot have unit type".to_owned()))?;
    values.insert(*result, builder.ins().iconst(ty, *value as i64));
    Ok(())
}

pub(super) fn lower_float_cast(
    _builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::FloatCast { result, source, source_type, target_type, .. } =
        instruction
    else {
        return Err(BackendEmitError(
            "Cranelift float cast instruction is not supported".to_owned(),
        ));
    };
    let source_value = values.get(source).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift float cast source {} is unavailable", source.0))
    })?;
    let _ = (result, source_type, target_type, source_value, values);
    Err(BackendEmitError(
        "Cranelift shared float conversion is not available for a non-literal value".to_owned(),
    ))
}

pub(super) fn lower_float_constant(
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::FloatConstant { result, ty, bits, .. } = instruction else {
        return Err(BackendEmitError(
            "Cranelift float constant instruction is not supported".to_owned(),
        ));
    };
    let value = match ty {
        crate::codegen::BackendValueType::Float { width: 32 } => {
            builder.ins().f32const(f32::from_bits(*bits as u32))
        }
        crate::codegen::BackendValueType::Float { width: 64 } => {
            builder.ins().f64const(f64::from_bits(*bits))
        }
        _ => {
            return Err(BackendEmitError(
                "Cranelift float constant requires a float type".to_owned(),
            ));
        }
    };
    values.insert(*result, value);
    Ok(())
}

pub(super) fn lower_memory_copy(
    module: &ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::MemoryCopy { destination, source, size, alignment, .. } = instruction
    else {
        return Err(BackendEmitError("Cranelift memory-copy instruction expected".to_owned()));
    };
    let destination = values.get(destination).copied().ok_or_else(|| {
        BackendEmitError(format!(
            "Cranelift memory-copy destination {} is unavailable",
            destination.0
        ))
    })?;
    let source = values.get(source).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift memory-copy source {} is unavailable", source.0))
    })?;
    let alignment = u8::try_from(*alignment)
        .map_err(|_| BackendEmitError("Cranelift memory-copy alignment exceeds u8".to_owned()))?;
    builder.emit_small_memory_copy(
        module.isa().frontend_config(),
        destination,
        source,
        u64::from(*size),
        alignment,
        alignment,
        false,
        MemFlagsData::new(),
    );
    Ok(())
}

pub(super) fn lower_scalar_loan(
    module: &ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    match instruction {
        BackendInstruction::ScalarLoanLoad { .. } => {
            lower_scalar_loan_load(module, builder, instruction, values)
        }
        BackendInstruction::ScalarLoanStore { .. } => {
            lower_scalar_loan_store(module, builder, instruction, values)
        }
        BackendInstruction::ScalarLoanMaterialize { .. } => {
            lower_scalar_loan_materialize(module, builder, instruction, values)
        }
        _ => Err(BackendEmitError("Cranelift scalar loan instruction expected".to_owned())),
    }
}

fn lower_scalar_loan_load(
    module: &ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::ScalarLoanLoad { result, address, ty, .. } = instruction else {
        return Err(BackendEmitError("Cranelift scalar loan load expected".to_owned()));
    };
    let address = values.get(address).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift scalar loan address {} is unavailable", address.0))
    })?;
    let load_type = super::scalar_type(module, *ty)?.ok_or_else(|| {
        BackendEmitError("Cranelift scalar loan load requires a scalar type".to_owned())
    })?;
    values.insert(*result, builder.ins().load(load_type, MemFlagsData::new(), address, 0));
    Ok(())
}

fn lower_scalar_loan_store(
    module: &ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::ScalarLoanStore { address, value, ty, .. } = instruction else {
        return Err(BackendEmitError("Cranelift scalar loan store expected".to_owned()));
    };
    let address = values.get(address).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift scalar loan address {} is unavailable", address.0))
    })?;
    let value = values.get(value).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift scalar loan value {} is unavailable", value.0))
    })?;
    let store_type = super::scalar_type(module, *ty)?.ok_or_else(|| {
        BackendEmitError("Cranelift scalar loan store requires a scalar type".to_owned())
    })?;
    if builder.func.dfg.value_type(value) != store_type {
        return Err(BackendEmitError("Cranelift scalar loan value type mismatch".to_owned()));
    }
    builder.ins().store(MemFlagsData::new(), value, address, 0);
    Ok(())
}

fn lower_scalar_loan_materialize(
    module: &ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::ScalarLoanMaterialize { result, value, ty, .. } = instruction else {
        return Err(BackendEmitError("Cranelift scalar loan materialization expected".to_owned()));
    };
    let value = values.get(value).copied().ok_or_else(|| {
        BackendEmitError(format!(
            "Cranelift scalar loan materialization value {} is unavailable",
            value.0
        ))
    })?;
    let scalar_type = super::scalar_type(module, *ty)?.ok_or_else(|| {
        BackendEmitError("Cranelift scalar loan materialization requires a scalar type".to_owned())
    })?;
    if builder.func.dfg.value_type(value) != scalar_type {
        return Err(BackendEmitError(
            "Cranelift scalar loan materialization type mismatch".to_owned(),
        ));
    }
    let slot = builder.func.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
        cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
        scalar_type.bytes(),
        0,
    ));
    let address = builder.ins().stack_addr(module.isa().pointer_type(), slot, 0);
    builder.ins().store(MemFlagsData::new(), value, address, 0);
    values.insert(*result, address);
    Ok(())
}
