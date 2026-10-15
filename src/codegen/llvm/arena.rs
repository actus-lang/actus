use std::collections::BTreeMap;

use llvm_sys::LLVMIntPredicate;
use llvm_sys::core::{
    LLVMAppendBasicBlockInContext, LLVMBuildAdd, LLVMBuildArrayAlloca, LLVMBuildCondBr,
    LLVMBuildGEP2, LLVMBuildICmp, LLVMBuildLoad2, LLVMBuildMemMove, LLVMBuildStore,
    LLVMBuildUnreachable, LLVMConstInt, LLVMGetBasicBlockParent, LLVMGetInsertBlock,
    LLVMInt8TypeInContext, LLVMInt32TypeInContext, LLVMInt64TypeInContext,
    LLVMPositionBuilderAtEnd, LLVMSetAlignment,
};
use llvm_sys::prelude::{LLVMBuilderRef, LLVMContextRef, LLVMValueRef};

use super::super::{BackendEmitError, BackendInstruction, BackendValueId};

pub(super) fn emit(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    pointer_width: u8,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    match instruction {
        BackendInstruction::ArenaConstruct { .. } => {
            emit_construct(context, builder, pointer_width, instruction, values)
        }
        BackendInstruction::ArenaPlace { .. } => {
            emit_place(context, builder, pointer_width, instruction, values)
        }
        _ => Err(BackendEmitError("LLVM arena instruction expected".to_owned())),
    }
}

fn emit_construct(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    pointer_width: u8,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let BackendInstruction::ArenaConstruct { result, capacity, pointer_width: width, .. } =
        instruction
    else {
        return Err(BackendEmitError("LLVM arena construction expected".to_owned()));
    };
    validate_pointer_width(pointer_width, *width)?;
    let total = capacity
        .checked_add(u32::from(*width))
        .ok_or_else(|| BackendEmitError("arena storage size overflow".to_owned()))?;
    let byte_type = unsafe { LLVMInt8TypeInContext(context) };
    let count = unsafe { LLVMConstInt(LLVMInt64TypeInContext(context), u64::from(total), 0) };
    let address =
        unsafe { LLVMBuildArrayAlloca(builder, byte_type, count, c"arena".as_ptr().cast()) };
    if address.is_null() {
        return Err(BackendEmitError("LLVM failed to allocate arena storage".to_owned()));
    }
    unsafe { LLVMSetAlignment(address, *width as u32) };
    let offset_address = byte_address(context, builder, address, *capacity)?;
    let offset_type = pointer_integer_type(context, *width)?;
    let zero = unsafe { LLVMConstInt(offset_type, 0, 0) };
    if unsafe { LLVMBuildStore(builder, zero, offset_address) }.is_null() {
        return Err(BackendEmitError("LLVM failed to initialize arena offset".to_owned()));
    }
    values.insert(*result, address);
    Ok(false)
}

fn emit_place(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    pointer_width: u8,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let BackendInstruction::ArenaPlace {
        result,
        arena,
        value,
        capacity,
        value_layout,
        alignment,
        pointer_width: width,
        ..
    } = instruction
    else {
        return Err(BackendEmitError("LLVM arena placement expected".to_owned()));
    };
    validate_pointer_width(pointer_width, *width)?;
    let arena = values
        .get(arena)
        .copied()
        .ok_or_else(|| BackendEmitError("LLVM arena value is unavailable".to_owned()))?;
    let value = values
        .get(value)
        .copied()
        .ok_or_else(|| BackendEmitError("LLVM arena placement value is unavailable".to_owned()))?;
    let state =
        place_state(context, builder, arena, *capacity, value_layout.size, *alignment, *width)?;
    publish_place(builder, &state)?;
    let destination = byte_address_value(context, builder, arena, state.aligned)?;
    let size =
        unsafe { LLVMConstInt(LLVMInt64TypeInContext(context), u64::from(value_layout.size), 0) };
    if unsafe { LLVMBuildMemMove(builder, destination, *alignment, value, *alignment, size) }
        .is_null()
    {
        return Err(BackendEmitError("LLVM failed to copy arena value".to_owned()));
    }
    values.insert(*result, destination);
    Ok(false)
}

struct PlacementState {
    offset_address: LLVMValueRef,
    aligned: LLVMValueRef,
    new_offset: LLVMValueRef,
}

fn place_state(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    arena: LLVMValueRef,
    capacity: u32,
    value_size: u32,
    alignment: u32,
    pointer_width: u8,
) -> Result<PlacementState, BackendEmitError> {
    let offset_address = byte_address(context, builder, arena, capacity)?;
    let offset_type = pointer_integer_type(context, pointer_width)?;
    let offset = unsafe {
        LLVMBuildLoad2(builder, offset_type, offset_address, c"arena.offset".as_ptr().cast())
    };
    if offset.is_null() {
        return Err(BackendEmitError("LLVM failed to load arena offset".to_owned()));
    }
    let alignment_minus_one =
        unsafe { LLVMConstInt(offset_type, u64::from(alignment.saturating_sub(1)), 0) };
    let rounded = unsafe {
        LLVMBuildAdd(builder, offset, alignment_minus_one, c"arena.rounded".as_ptr().cast())
    };
    let inverse = unsafe {
        llvm_sys::core::LLVMBuildNot(builder, alignment_minus_one, c"arena.mask".as_ptr().cast())
    };
    let aligned = unsafe {
        llvm_sys::core::LLVMBuildAnd(builder, rounded, inverse, c"arena.aligned".as_ptr().cast())
    };
    let size = unsafe { LLVMConstInt(offset_type, u64::from(value_size), 0) };
    let new_offset =
        unsafe { LLVMBuildAdd(builder, aligned, size, c"arena.new_offset".as_ptr().cast()) };
    let limit = unsafe { LLVMConstInt(offset_type, u64::from(capacity), 0) };
    let fits = unsafe {
        LLVMBuildICmp(
            builder,
            LLVMIntPredicate::LLVMIntULE,
            new_offset,
            limit,
            c"arena.fits".as_ptr().cast(),
        )
    };
    if fits.is_null() {
        return Err(BackendEmitError("LLVM failed to check arena capacity".to_owned()));
    }
    let parent = unsafe { LLVMGetBasicBlockParent(LLVMGetInsertBlock(builder)) };
    let ok = unsafe { LLVMAppendBasicBlockInContext(context, parent, c"arena.ok".as_ptr().cast()) };
    let trap =
        unsafe { LLVMAppendBasicBlockInContext(context, parent, c"arena.trap".as_ptr().cast()) };
    unsafe { LLVMBuildCondBr(builder, fits, ok, trap) };
    unsafe { LLVMPositionBuilderAtEnd(builder, trap) };
    unsafe { LLVMBuildUnreachable(builder) };
    unsafe { LLVMPositionBuilderAtEnd(builder, ok) };
    Ok(PlacementState { offset_address, aligned, new_offset })
}

fn publish_place(builder: LLVMBuilderRef, state: &PlacementState) -> Result<(), BackendEmitError> {
    if unsafe { LLVMBuildStore(builder, state.new_offset, state.offset_address) }.is_null() {
        return Err(BackendEmitError("LLVM failed to publish arena offset".to_owned()));
    }
    Ok(())
}

fn byte_address(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    base: LLVMValueRef,
    offset: u32,
) -> Result<LLVMValueRef, BackendEmitError> {
    byte_address_value(context, builder, base, unsafe {
        LLVMConstInt(LLVMInt64TypeInContext(context), u64::from(offset), 0)
    })
}

fn byte_address_value(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    base: LLVMValueRef,
    offset: LLVMValueRef,
) -> Result<LLVMValueRef, BackendEmitError> {
    let address = unsafe {
        LLVMBuildGEP2(
            builder,
            LLVMInt8TypeInContext(context),
            base,
            &offset as *const _ as *mut _,
            1,
            c"arena.address".as_ptr().cast(),
        )
    };
    (!address.is_null())
        .then_some(address)
        .ok_or_else(|| BackendEmitError("LLVM failed to address arena storage".to_owned()))
}

fn pointer_integer_type(
    context: LLVMContextRef,
    width: u8,
) -> Result<llvm_sys::prelude::LLVMTypeRef, BackendEmitError> {
    match width {
        4 => Ok(unsafe { LLVMInt32TypeInContext(context) }),
        8 => Ok(unsafe { LLVMInt64TypeInContext(context) }),
        _ => Err(BackendEmitError("LLVM arena pointer width must be 4 or 8".to_owned())),
    }
}

fn validate_pointer_width(actual: u8, declared: u8) -> Result<(), BackendEmitError> {
    if actual == declared.saturating_mul(8) {
        Ok(())
    } else {
        Err(BackendEmitError("LLVM arena pointer width mismatches target".to_owned()))
    }
}
