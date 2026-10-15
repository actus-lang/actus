use std::collections::BTreeMap;

use super::declarations::FunctionDeclaration;
use llvm_sys::core::{
    LLVMBuildAlloca, LLVMBuildGEP2, LLVMBuildLoad2, LLVMBuildMemMove, LLVMBuildMemSet,
    LLVMBuildStore, LLVMBuildTrunc, LLVMBuildZExt, LLVMConstInt, LLVMGetIntTypeWidth,
    LLVMInt8TypeInContext, LLVMInt64TypeInContext, LLVMSetAlignment, LLVMTypeOf,
};
use llvm_sys::prelude::{LLVMBuilderRef, LLVMContextRef, LLVMTypeRef, LLVMValueRef};

use super::super::{BackendEmitError, BackendInstruction, BackendValueId, BackendValueType};

pub(super) fn emit_aggregate_construct(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    instruction: &BackendInstruction,
    declarations: &BTreeMap<String, FunctionDeclaration>,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let BackendInstruction::AggregateConstruct { result, layout, fields, .. } = instruction else {
        return Err(BackendEmitError("LLVM aggregate construction expected".to_owned()));
    };
    let aggregate_type =
        super::values::llvm_type(context, BackendValueType::Aggregate { layout: *layout })?;
    let address =
        allocate_aggregate_storage(context, builder, *layout, aggregate_type, declarations)?;
    if address.is_null() {
        return Err(BackendEmitError("LLVM failed to allocate aggregate value".to_owned()));
    }
    initialize_aggregate_storage(context, builder, address, *layout, fields, values)?;
    values.insert(*result, address);
    Ok(false)
}

fn allocate_aggregate_storage(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    layout: crate::codegen::BackendLayout,
    aggregate_type: LLVMTypeRef,
    declarations: &BTreeMap<String, FunctionDeclaration>,
) -> Result<LLVMValueRef, BackendEmitError> {
    if layout.size < crate::codegen::LARGE_AGGREGATE_THRESHOLD {
        return Ok(unsafe {
            LLVMBuildAlloca(builder, aggregate_type, c"aggregate.construct".as_ptr().cast())
        });
    }
    let declaration = declarations
        .get(crate::runtime::AGGREGATE_ALLOCATE_SYMBOL)
        .ok_or_else(|| BackendEmitError("large aggregate allocator is unavailable".to_owned()))?;
    let size = unsafe { LLVMConstInt(LLVMInt64TypeInContext(context), u64::from(layout.size), 0) };
    let alignment =
        unsafe { LLVMConstInt(LLVMInt64TypeInContext(context), u64::from(layout.alignment), 0) };
    let mut arguments = [size, alignment];
    Ok(unsafe {
        llvm_sys::core::LLVMBuildCall2(
            builder,
            declaration.function_type,
            declaration.value,
            arguments.as_mut_ptr(),
            2,
            c"aggregate.allocate".as_ptr().cast(),
        )
    })
}

fn initialize_aggregate_storage(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    address: LLVMValueRef,
    layout: crate::codegen::BackendLayout,
    fields: &[crate::codegen::BackendAggregateField],
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<(), BackendEmitError> {
    unsafe { LLVMSetAlignment(address, layout.alignment) };
    let zero = unsafe { LLVMConstInt(LLVMInt8TypeInContext(context), 0, 0) };
    let size = unsafe { LLVMConstInt(LLVMInt64TypeInContext(context), u64::from(layout.size), 0) };
    if unsafe { LLVMBuildMemSet(builder, address, zero, size, layout.alignment) }.is_null() {
        return Err(BackendEmitError("LLVM failed to zero-initialize aggregate value".to_owned()));
    }
    for field in fields {
        initialize_aggregate_field(context, builder, address, field, values)?;
    }
    Ok(())
}

fn initialize_aggregate_field(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    address: LLVMValueRef,
    field: &crate::codegen::BackendAggregateField,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<(), BackendEmitError> {
    let value = values.get(&field.value).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM aggregate field value {} is unavailable", field.value.0))
    })?;
    let destination = field_address(context, builder, address, field.offset)?;
    if matches!(
        field.ty,
        BackendValueType::Aggregate { .. }
            | BackendValueType::Enum { .. }
            | BackendValueType::Region { .. }
    ) {
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
    } else {
        let value = coerce_scalar_value(context, builder, value, field.ty)?;
        if unsafe { LLVMBuildStore(builder, value, destination) }.is_null() {
            return Err(BackendEmitError("LLVM failed to store aggregate field".to_owned()));
        }
    }
    Ok(())
}

pub(super) fn emit_aggregate_instruction(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    pointer_width: u8,
    instruction: &BackendInstruction,
    declarations: &BTreeMap<String, FunctionDeclaration>,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    match instruction {
        BackendInstruction::ArenaConstruct { .. } | BackendInstruction::ArenaPlace { .. } => {
            super::arena::emit(context, builder, pointer_width, instruction, values)
        }
        BackendInstruction::AggregateConstruct { .. } => {
            emit_aggregate_construct(context, builder, instruction, declarations, values)
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
    if matches!(
        ty,
        BackendValueType::Aggregate { .. }
            | BackendValueType::Enum { .. }
            | BackendValueType::Region { .. }
    ) {
        // Aggregate values are represented by their storage address in the
        // neutral contract. Preserve that address for nested indexed stores so
        // mutations update the owning aggregate instead of a detached copy.
        values.insert(*result, address);
    } else {
        let value_type = super::values::llvm_type(context, *ty)?;
        let value = unsafe {
            LLVMBuildLoad2(builder, value_type, address, c"aggregate.field.load".as_ptr().cast())
        };
        if value.is_null() {
            return Err(BackendEmitError("LLVM failed to load aggregate field".to_owned()));
        }
        values.insert(*result, value);
    }
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
    let copy_layout = match ty {
        BackendValueType::Aggregate { layout } | BackendValueType::Region { layout } => *layout,
        BackendValueType::Enum { layout } => layout.layout,
        _ => {
            let value = coerce_scalar_value(context, builder, value, *ty)?;
            if unsafe { LLVMBuildStore(builder, value, address) }.is_null() {
                return Err(BackendEmitError("LLVM failed to store aggregate field".to_owned()));
            }
            return Ok(false);
        }
    };
    {
        let size = unsafe {
            LLVMConstInt(LLVMInt64TypeInContext(context), u64::from(copy_layout.size), 0)
        };
        let copy = unsafe {
            LLVMBuildMemMove(
                builder,
                address,
                copy_layout.alignment,
                value,
                copy_layout.alignment,
                size,
            )
        };
        if copy.is_null() {
            return Err(BackendEmitError("LLVM failed to copy aggregate field".to_owned()));
        }
    }
    let _ = ty;
    Ok(false)
}

fn coerce_scalar_value(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    value: LLVMValueRef,
    ty: BackendValueType,
) -> Result<LLVMValueRef, BackendEmitError> {
    let target = super::values::llvm_type(context, ty)?;
    let source_width = unsafe { LLVMGetIntTypeWidth(LLVMTypeOf(value)) };
    let target_width = unsafe { LLVMGetIntTypeWidth(target) };
    if source_width == 0 || target_width == 0 || source_width == target_width {
        return Ok(value);
    }
    let value = if source_width < target_width {
        unsafe { LLVMBuildZExt(builder, value, target, c"aggregate.field.zext".as_ptr().cast()) }
    } else {
        unsafe { LLVMBuildTrunc(builder, value, target, c"aggregate.field.trunc".as_ptr().cast()) }
    };
    (!value.is_null())
        .then_some(value)
        .ok_or_else(|| BackendEmitError("LLVM failed to coerce aggregate field value".to_owned()))
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
