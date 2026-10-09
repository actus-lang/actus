use std::collections::BTreeMap;

use llvm_sys::LLVMTypeKind::{LLVMIntegerTypeKind, LLVMPointerTypeKind};
use llvm_sys::core::{
    LLVMBuildAlloca, LLVMBuildBr, LLVMBuildCondBr, LLVMBuildGEP2, LLVMBuildLoad2, LLVMBuildMemMove,
    LLVMBuildRetVoid, LLVMBuildStore, LLVMBuildUnreachable, LLVMConstInt, LLVMConstNull,
    LLVMGetParam, LLVMInt8TypeInContext, LLVMInt16TypeInContext, LLVMInt32TypeInContext,
    LLVMInt64TypeInContext, LLVMPositionBuilderAtEnd, LLVMSetAlignment, LLVMTypeOf,
};
use llvm_sys::core::{LLVMGetIntTypeWidth, LLVMGetTypeKind};
use llvm_sys::prelude::{LLVMBasicBlockRef, LLVMBuilderRef, LLVMValueRef};

use super::super::model::LoweredCleanupInstruction;
use super::super::{
    BackendBlockId, BackendEmitError, BackendEnumLayout, BackendFunction, BackendInstruction,
    BackendLayout, BackendValueId, BackendValueType,
};
use super::calls::emit_call;
use super::declarations::FunctionDeclaration;
use super::returns::emit_return;

pub(super) fn emit_blocks(
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    function: &BackendFunction,
    declaration: FunctionDeclaration,
    declarations: &BTreeMap<String, FunctionDeclaration>,
    blocks: &BTreeMap<BackendBlockId, LLVMBasicBlockRef>,
) -> Result<(), BackendEmitError> {
    let mut values = parameter_values(declaration);
    for block in &function.blocks {
        let llvm_block = block_value(blocks, block.id)?;
        unsafe { LLVMPositionBuilderAtEnd(builder, llvm_block) };
        let mut terminated = false;
        for instruction in &block.instructions {
            if terminated {
                return Err(BackendEmitError(
                    "LLVM block has instructions after a terminator".to_owned(),
                ));
            }
            terminated = emit_instruction(
                context,
                builder,
                function,
                declaration,
                instruction,
                declarations,
                blocks,
                &mut values,
            )?;
        }
        if !terminated {
            if matches!(function.return_value.ty, BackendValueType::Unit) {
                unsafe { LLVMBuildRetVoid(builder) };
            } else {
                return Err(BackendEmitError(
                    "LLVM function block has no supported return".to_owned(),
                ));
            }
        }
    }
    Ok(())
}

fn parameter_values(declaration: FunctionDeclaration) -> BTreeMap<BackendValueId, LLVMValueRef> {
    (0..declaration.parameter_count)
        .map(|index| {
            let value = unsafe { LLVMGetParam(declaration.value, index as u32) };
            (BackendValueId(index as u32), value)
        })
        .collect()
}

fn emit_instruction(
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    function: &BackendFunction,
    declaration: FunctionDeclaration,
    instruction: &BackendInstruction,
    declarations: &BTreeMap<String, FunctionDeclaration>,
    blocks: &BTreeMap<BackendBlockId, LLVMBasicBlockRef>,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    match instruction {
        BackendInstruction::Return { value, .. } => {
            emit_return(builder, function, declaration, *value, values)?;
            Ok(true)
        }
        BackendInstruction::Call { callee, arguments, result, .. } => {
            emit_call(builder, callee.as_str(), arguments, *result, declarations, values)?;
            Ok(false)
        }
        BackendInstruction::Branch { condition, then_block, else_block, .. } => {
            emit_branch(builder, *condition, *then_block, *else_block, blocks, values)?;
            Ok(true)
        }
        BackendInstruction::Jump { target, .. } => {
            emit_jump(builder, *target, blocks)?;
            Ok(true)
        }
        BackendInstruction::Trap { .. } => {
            unsafe { LLVMBuildUnreachable(builder) };
            Ok(true)
        }
        BackendInstruction::Fail { failure, .. } => {
            emit_failure(context, builder, function, declaration, failure, values)?;
            Ok(true)
        }
        BackendInstruction::Cleanup { action, .. } => emit_cleanup(action),
        BackendInstruction::MemoryCopy { destination, source, size, alignment, .. } => {
            emit_memory_copy(context, builder, *destination, *source, *size, *alignment, values)
        }
    }
}

fn emit_failure(
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    function: &BackendFunction,
    declaration: FunctionDeclaration,
    failure: &super::super::BackendFailure,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<(), BackendEmitError> {
    validate_failure_return(function, failure.layout)?;
    let enum_type = super::values::llvm_type(context, function.return_value.ty)?;
    let temporary = allocate_failure_storage(builder, enum_type, failure.layout.layout)?;
    store_discriminant(context, builder, temporary, failure)?;
    store_payload(context, builder, temporary, failure, values)?;
    let value = load_failure(builder, enum_type, temporary)?;
    finish_failure(builder, declaration, value);
    Ok(())
}

fn validate_failure_return(
    function: &BackendFunction,
    layout: BackendEnumLayout,
) -> Result<(), BackendEmitError> {
    let return_layout = match function.return_value.ty {
        BackendValueType::Aggregate { layout } | BackendValueType::Region { layout } => layout,
        _ => {
            return Err(BackendEmitError(
                "LLVM typed failure requires an aggregate enum return type".to_owned(),
            ));
        }
    };
    if return_layout != layout.layout {
        return Err(BackendEmitError(
            "LLVM typed failure enum layout does not match the function return layout".to_owned(),
        ));
    }
    if layout.niche_pointer {
        return Err(BackendEmitError(
            "LLVM niche-pointer failure lowering is not implemented".to_owned(),
        ));
    }
    Ok(())
}

fn allocate_failure_storage(
    builder: LLVMBuilderRef,
    enum_type: llvm_sys::prelude::LLVMTypeRef,
    layout: BackendLayout,
) -> Result<LLVMValueRef, BackendEmitError> {
    let temporary = unsafe { LLVMBuildAlloca(builder, enum_type, b"failure\0".as_ptr().cast()) };
    if temporary.is_null() {
        return Err(BackendEmitError("LLVM failed to allocate typed failure value".to_owned()));
    }
    unsafe { LLVMSetAlignment(temporary, layout.alignment) };
    let zero = unsafe { LLVMConstNull(enum_type) };
    if unsafe { LLVMBuildStore(builder, zero, temporary) }.is_null() {
        return Err(BackendEmitError("LLVM failed to initialize typed failure value".to_owned()));
    }
    Ok(temporary)
}

fn store_discriminant(
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    temporary: LLVMValueRef,
    failure: &super::super::BackendFailure,
) -> Result<(), BackendEmitError> {
    let byte_type = unsafe { LLVMInt8TypeInContext(context) };
    let ty = discriminant_type(context, failure.layout)?;
    let value = unsafe { LLVMConstInt(ty, u64::from(failure.discriminant), 0) };
    let address = byte_offset(
        builder,
        context,
        temporary,
        byte_type,
        failure.layout.discriminant_offset,
        "failure.discriminant",
    )?;
    if unsafe { LLVMBuildStore(builder, value, address) }.is_null() {
        return Err(BackendEmitError("LLVM failed to store enum discriminant".to_owned()));
    }
    Ok(())
}

fn store_payload(
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    temporary: LLVMValueRef,
    failure: &super::super::BackendFailure,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<(), BackendEmitError> {
    let (Some(payload_id), Some(payload_layout)) = (failure.payload, failure.layout.payload) else {
        return Ok(());
    };
    let payload = values.get(&payload_id).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM failure payload {} is unavailable", payload_id.0))
    })?;
    validate_integer_payload(payload, payload_layout)?;
    let byte_type = unsafe { LLVMInt8TypeInContext(context) };
    let address = byte_offset(
        builder,
        context,
        temporary,
        byte_type,
        failure.layout.payload_offset,
        "failure.payload",
    )?;
    if unsafe { LLVMBuildStore(builder, payload, address) }.is_null() {
        return Err(BackendEmitError("LLVM failed to store enum payload".to_owned()));
    }
    Ok(())
}

fn load_failure(
    builder: LLVMBuilderRef,
    enum_type: llvm_sys::prelude::LLVMTypeRef,
    temporary: LLVMValueRef,
) -> Result<LLVMValueRef, BackendEmitError> {
    let value = unsafe {
        LLVMBuildLoad2(builder, enum_type, temporary, b"failure.value\0".as_ptr().cast())
    };
    (!value.is_null())
        .then_some(value)
        .ok_or_else(|| BackendEmitError("LLVM failed to load typed failure value".to_owned()))
}

fn finish_failure(builder: LLVMBuilderRef, declaration: FunctionDeclaration, value: LLVMValueRef) {
    if declaration.return_slot {
        let slot = unsafe { LLVMGetParam(declaration.value, 0) };
        unsafe {
            LLVMBuildStore(builder, value, slot);
            LLVMBuildRetVoid(builder);
        }
    } else {
        unsafe { llvm_sys::core::LLVMBuildRet(builder, value) };
    }
}

fn discriminant_type(
    context: llvm_sys::prelude::LLVMContextRef,
    layout: BackendEnumLayout,
) -> Result<llvm_sys::prelude::LLVMTypeRef, BackendEmitError> {
    Ok(match layout.discriminant_size {
        1 => unsafe { LLVMInt8TypeInContext(context) },
        2 => unsafe { LLVMInt16TypeInContext(context) },
        4 => unsafe { LLVMInt32TypeInContext(context) },
        8 => unsafe { LLVMInt64TypeInContext(context) },
        _ => return Err(BackendEmitError("LLVM enum discriminant width is invalid".to_owned())),
    })
}

fn byte_offset(
    builder: LLVMBuilderRef,
    context: llvm_sys::prelude::LLVMContextRef,
    pointer: LLVMValueRef,
    byte_type: llvm_sys::prelude::LLVMTypeRef,
    offset: u32,
    name: &str,
) -> Result<LLVMValueRef, BackendEmitError> {
    let name = std::ffi::CString::new(name)
        .map_err(|_| BackendEmitError("LLVM failure address name contains NUL".to_owned()))?;
    let mut index = unsafe { LLVMConstInt(LLVMInt64TypeInContext(context), u64::from(offset), 0) };
    let address =
        unsafe { LLVMBuildGEP2(builder, byte_type, pointer, &mut index, 1, name.as_ptr()) };
    (!address.is_null())
        .then_some(address)
        .ok_or_else(|| BackendEmitError("LLVM failed to address typed failure field".to_owned()))
}

fn validate_integer_payload(
    payload: LLVMValueRef,
    layout: super::super::BackendLayout,
) -> Result<(), BackendEmitError> {
    if unsafe { LLVMGetTypeKind(LLVMTypeOf(payload)) } != LLVMIntegerTypeKind {
        return Err(BackendEmitError(
            "LLVM typed failure currently requires an integer payload".to_owned(),
        ));
    }
    let width = unsafe { LLVMGetIntTypeWidth(LLVMTypeOf(payload)) };
    let bytes = u32::from(width.div_ceil(8));
    if bytes > layout.size {
        return Err(BackendEmitError(
            "LLVM typed failure payload exceeds its declared layout".to_owned(),
        ));
    }
    Ok(())
}

fn emit_memory_copy(
    context: llvm_sys::prelude::LLVMContextRef,
    builder: LLVMBuilderRef,
    destination_id: BackendValueId,
    source_id: BackendValueId,
    size: u32,
    alignment: u32,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let destination = pointer_value(destination_id, values, "destination")?;
    let source = pointer_value(source_id, values, "source")?;
    let length = unsafe { LLVMConstInt(LLVMInt64TypeInContext(context), u64::from(size), 0) };
    let copy =
        unsafe { LLVMBuildMemMove(builder, destination, alignment, source, alignment, length) };
    (!copy.is_null())
        .then_some(false)
        .ok_or_else(|| BackendEmitError("LLVM failed to build memory copy".to_owned()))
}

fn pointer_value(
    id: BackendValueId,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
    role: &str,
) -> Result<LLVMValueRef, BackendEmitError> {
    let value = values.get(&id).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM memory-copy {role} value {} is unavailable", id.0))
    })?;
    let kind = unsafe { LLVMGetTypeKind(LLVMTypeOf(value)) };
    (kind == LLVMPointerTypeKind)
        .then_some(value)
        .ok_or_else(|| BackendEmitError(format!("LLVM memory-copy {role} must be a pointer")))
}

fn emit_cleanup(action: &LoweredCleanupInstruction) -> Result<bool, BackendEmitError> {
    if matches!(action, LoweredCleanupInstruction::EndBorrow { .. }) {
        return Ok(false);
    }
    Err(BackendEmitError(format!(
        "LLVM cleanup action `{}` requires a runtime bridge lowering",
        cleanup_name(action)
    )))
}

fn cleanup_name(action: &LoweredCleanupInstruction) -> &'static str {
    match action {
        LoweredCleanupInstruction::EndBorrow { .. } => "end-borrow",
        LoweredCleanupInstruction::DropPayloadField { .. } => "drop-payload-field",
        LoweredCleanupInstruction::DropBinding { .. } => "drop-binding",
        LoweredCleanupInstruction::DropBindingFields { .. } => "drop-binding-fields",
        LoweredCleanupInstruction::ResetArena { .. } => "reset-arena",
    }
}

fn emit_branch(
    builder: LLVMBuilderRef,
    condition_id: BackendValueId,
    then_id: BackendBlockId,
    else_id: BackendBlockId,
    blocks: &BTreeMap<BackendBlockId, LLVMBasicBlockRef>,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<(), BackendEmitError> {
    let condition = values.get(&condition_id).copied().ok_or_else(|| {
        BackendEmitError(format!("LLVM branch value {} is unavailable", condition_id.0))
    })?;
    let then_block = block_value(blocks, then_id)?;
    let else_block = block_value(blocks, else_id)?;
    unsafe { LLVMBuildCondBr(builder, condition, then_block, else_block) };
    Ok(())
}

fn emit_jump(
    builder: LLVMBuilderRef,
    target: BackendBlockId,
    blocks: &BTreeMap<BackendBlockId, LLVMBasicBlockRef>,
) -> Result<(), BackendEmitError> {
    let target = block_value(blocks, target)?;
    unsafe { LLVMBuildBr(builder, target) };
    Ok(())
}

fn block_value(
    blocks: &BTreeMap<BackendBlockId, LLVMBasicBlockRef>,
    id: BackendBlockId,
) -> Result<LLVMBasicBlockRef, BackendEmitError> {
    blocks
        .get(&id)
        .copied()
        .ok_or_else(|| BackendEmitError(format!("LLVM block {} is unavailable", id.0)))
}
