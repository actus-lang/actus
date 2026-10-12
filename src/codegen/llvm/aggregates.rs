use std::collections::BTreeMap;

use llvm_sys::core::{
    LLVMBuildAlloca, LLVMBuildGEP2, LLVMBuildLoad2, LLVMBuildMemMove, LLVMBuildMemSet,
    LLVMBuildStore, LLVMConstInt, LLVMInt8TypeInContext, LLVMInt64TypeInContext, LLVMSetAlignment,
};
use llvm_sys::prelude::{LLVMBuilderRef, LLVMContextRef, LLVMValueRef};

use super::super::{BackendEmitError, BackendInstruction, BackendValueId, BackendValueType};

pub(super) fn emit_aggregate_construct(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let BackendInstruction::AggregateConstruct { result, layout, fields, .. } = instruction else {
        return Err(BackendEmitError("LLVM aggregate construction expected".to_owned()));
    };
    let aggregate_type =
        super::values::llvm_type(context, BackendValueType::Aggregate { layout: *layout })?;
    let address =
        unsafe { LLVMBuildAlloca(builder, aggregate_type, c"aggregate.construct".as_ptr().cast()) };
    if address.is_null() {
        return Err(BackendEmitError("LLVM failed to allocate aggregate value".to_owned()));
    }
    unsafe { LLVMSetAlignment(address, layout.alignment) };
    let zero = unsafe { LLVMConstInt(LLVMInt8TypeInContext(context), 0, 0) };
    let size = unsafe { LLVMConstInt(LLVMInt64TypeInContext(context), u64::from(layout.size), 0) };
    if unsafe { LLVMBuildMemSet(builder, address, zero, size, layout.alignment) }.is_null() {
        return Err(BackendEmitError("LLVM failed to zero-initialize aggregate value".to_owned()));
    }
    for field in fields {
        let value = values.get(&field.value).copied().ok_or_else(|| {
            BackendEmitError(format!("LLVM aggregate field value {} is unavailable", field.value.0))
        })?;
        let destination = field_address(context, builder, address, field.offset)?;
        if matches!(field.ty, BackendValueType::Aggregate { .. } | BackendValueType::Region { .. })
        {
            let size = unsafe {
                LLVMConstInt(LLVMInt64TypeInContext(context), u64::from(field.layout.size), 0)
            };
            let copy = unsafe {
                LLVMBuildMemMove(
                    builder,
                    destination,
                    field.layout.alignment,
                    value,
                    field.layout.alignment,
                    size,
                )
            };
            if copy.is_null() {
                return Err(BackendEmitError("LLVM failed to copy aggregate field".to_owned()));
            }
        } else if unsafe { LLVMBuildStore(builder, value, destination) }.is_null() {
            return Err(BackendEmitError("LLVM failed to store aggregate field".to_owned()));
        }
    }
    values.insert(*result, address);
    Ok(false)
}

pub(super) fn emit_aggregate_instruction(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    pointer_width: u8,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    match instruction {
        BackendInstruction::ArenaConstruct { .. } | BackendInstruction::ArenaPlace { .. } => {
            super::arena::emit(context, builder, pointer_width, instruction, values)
        }
        BackendInstruction::AggregateConstruct { .. } => {
            emit_aggregate_construct(context, builder, instruction, values)
        }
        BackendInstruction::AggregateFieldLoad { .. } => {
            emit_aggregate_field_load(context, builder, instruction, values)
        }
        BackendInstruction::PackedFieldLoad { .. } => {
            super::packed::emit_packed_field_load(context, builder, instruction, values)
        }
        BackendInstruction::PackedFieldStore { .. } => {
            super::packed::emit_packed_field_store(context, builder, instruction, values)
        }
        BackendInstruction::ArrayElementLoad { .. } => {
            super::arrays::emit_array_element_load(context, builder, instruction, values)
        }
        BackendInstruction::ArrayElementStore { .. } => {
            super::arrays::emit_array_element_store(context, builder, instruction, values)
        }
        _ => Err(BackendEmitError("LLVM aggregate instruction expected".to_owned())),
    }
}

pub(super) fn emit_aggregate_field_load(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let BackendInstruction::AggregateFieldLoad { result, base, offset, ty, .. } = instruction
    else {
        return Err(BackendEmitError("LLVM aggregate field load expected".to_owned()));
    };
    let base = values.get(base).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM aggregate base {} is unavailable", base.0))
    })?;
    let address = field_address(context, builder, base, *offset)?;
    let value_type = super::values::llvm_type(context, *ty)?;
    let value = unsafe {
        LLVMBuildLoad2(builder, value_type, address, c"aggregate.field.load".as_ptr().cast())
    };
    if value.is_null() {
        return Err(BackendEmitError("LLVM failed to load aggregate field".to_owned()));
    }
    values.insert(*result, value);
    Ok(false)
}

pub(super) fn emit_aggregate_field_store(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    instruction: &BackendInstruction,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let BackendInstruction::AggregateFieldStore { base, offset, value, ty, .. } = instruction
    else {
        return Err(BackendEmitError("LLVM aggregate field store expected".to_owned()));
    };
    let base = values.get(base).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM aggregate base {} is unavailable", base.0))
    })?;
    let value = values.get(value).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM aggregate field value {} is unavailable", value.0))
    })?;
    let address = field_address(context, builder, base, *offset)?;
    if unsafe { LLVMBuildStore(builder, value, address) }.is_null() {
        return Err(BackendEmitError("LLVM failed to store aggregate field".to_owned()));
    }
    let _ = ty;
    Ok(false)
}

fn field_address(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    base: LLVMValueRef,
    offset: u32,
) -> Result<LLVMValueRef, BackendEmitError> {
    let index = unsafe { LLVMConstInt(LLVMInt64TypeInContext(context), u64::from(offset), 0) };
    let address = unsafe {
        LLVMBuildGEP2(
            builder,
            LLVMInt8TypeInContext(context),
            base,
            &index as *const _ as *mut _,
            1,
            c"aggregate.field".as_ptr().cast(),
        )
    };
    (!address.is_null())
        .then_some(address)
        .ok_or_else(|| BackendEmitError("LLVM failed to address aggregate field".to_owned()))
}
