use std::collections::BTreeMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData, TrapCode, condcodes::IntCC};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::Module;
use cranelift_object::ObjectModule;

use super::super::native::BackendEmitError;
use crate::codegen::{BackendInstruction, BackendValueId};

pub(super) fn lower(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    match instruction {
        BackendInstruction::ArenaConstruct { .. } => {
            lower_construct(module, builder, instruction, values)
        }
        BackendInstruction::ArenaPlace { .. } => lower_place(module, builder, instruction, values),
        _ => Err(BackendEmitError("Cranelift arena instruction expected".to_owned())),
    }
}

fn lower_construct(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::ArenaConstruct { result, capacity, pointer_width, .. } = instruction
    else {
        return Err(BackendEmitError("Cranelift arena construction expected".to_owned()));
    };
    let size = capacity
        .checked_add(u32::from(*pointer_width))
        .ok_or_else(|| BackendEmitError("arena storage size overflow".to_owned()))?;
    let pointer_type = module.isa().pointer_type();
    let slot = builder.func.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
        cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
        size,
        pointer_type.bytes().trailing_zeros() as u8,
    ));
    let address = builder.ins().stack_addr(pointer_type, slot, 0);
    let offset_address = builder.ins().iadd_imm_s(address, i64::from(*capacity));
    let zero = builder.ins().iconst(pointer_type, 0);
    builder.ins().store(MemFlagsData::new(), zero, offset_address, 0);
    values.insert(*result, address);
    Ok(())
}

fn lower_place(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::ArenaPlace {
        result,
        arena,
        value,
        capacity,
        value_layout,
        alignment,
        ..
    } = instruction
    else {
        return Err(BackendEmitError("Cranelift arena placement expected".to_owned()));
    };
    let arena = values
        .get(arena)
        .copied()
        .ok_or_else(|| BackendEmitError("Cranelift arena value is unavailable".to_owned()))?;
    let value = values.get(value).copied().ok_or_else(|| {
        BackendEmitError("Cranelift arena placement value is unavailable".to_owned())
    })?;
    let pointer_type = module.isa().pointer_type();
    let offset_address = builder.ins().iadd_imm_s(arena, i64::from(*capacity));
    let offset = builder.ins().load(pointer_type, MemFlagsData::new(), offset_address, 0);
    let aligned = aligned_offset(builder, offset, *alignment, pointer_type);
    let new_offset = builder.ins().iadd_imm_s(aligned, i64::from(value_layout.size));
    let capacity_value = builder.ins().iconst(pointer_type, i64::from(*capacity));
    let fits = builder.ins().icmp(IntCC::UnsignedLessThanOrEqual, new_offset, capacity_value);
    let ok_block = builder.create_block();
    let trap_block = builder.create_block();
    builder.ins().brif(fits, ok_block, &[], trap_block, &[]);
    builder.switch_to_block(trap_block);
    builder.ins().trap(TrapCode::HEAP_OUT_OF_BOUNDS);
    builder.seal_block(trap_block);
    builder.switch_to_block(ok_block);
    builder.ins().store(MemFlagsData::new(), new_offset, offset_address, 0);
    let destination = builder.ins().iadd(arena, aligned);
    builder.emit_small_memory_copy(
        module.isa().frontend_config(),
        destination,
        value,
        u64::from(value_layout.size),
        value_layout.alignment.trailing_zeros() as u8,
        value_layout.alignment.trailing_zeros() as u8,
        false,
        MemFlagsData::new(),
    );
    builder.seal_block(ok_block);
    values.insert(*result, destination);
    Ok(())
}

fn aligned_offset(
    builder: &mut FunctionBuilder<'_>,
    offset: cranelift_codegen::ir::Value,
    alignment: u32,
    pointer_type: cranelift_codegen::ir::Type,
) -> cranelift_codegen::ir::Value {
    let alignment_minus_one =
        builder.ins().iconst(pointer_type, i64::from(alignment.saturating_sub(1)));
    let rounded = builder.ins().iadd(offset, alignment_minus_one);
    let mask = builder.ins().bnot(alignment_minus_one);
    builder.ins().band(rounded, mask)
}
