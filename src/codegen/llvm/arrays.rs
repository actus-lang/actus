use std::collections::BTreeMap;

use llvm_sys::core::{
    LLVMBuildGEP2, LLVMBuildLoad2, LLVMBuildMemMove, LLVMBuildMul, LLVMBuildStore, LLVMBuildZExt,
    LLVMConstInt, LLVMInt8TypeInContext, LLVMInt64TypeInContext,
};
use llvm_sys::prelude::{LLVMBuilderRef, LLVMContextRef, LLVMValueRef};

use super::super::{BackendEmitError, BackendInstruction, BackendValueId, BackendValueType};

pub(super) fn emit_array_element_load(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let BackendInstruction::ArrayElementLoad {
        result,
        base,
        index,
        element_layout,
        element_type,
        ..
    } = instruction
    else {
        return Err(BackendEmitError("LLVM array element load expected".to_owned()));
    };
    let base = values
        .get(base)
        .copied()
        .ok_or_else(|| BackendEmitError(format!("LLVM array base {} is unavailable", base.0)))?;
    let index = values
        .get(index)
        .copied()
        .ok_or_else(|| BackendEmitError(format!("LLVM array index {} is unavailable", index.0)))?;
    let address = array_element_address(context, builder, base, index, *element_layout)?;
    if matches!(element_type, BackendValueType::Aggregate { .. } | BackendValueType::Region { .. })
    {
        values.insert(*result, address);
        return Ok(false);
    }
    let value_type = super::values::llvm_type(context, *element_type)?;
    let value = unsafe {
        LLVMBuildLoad2(builder, value_type, address, c"array.element.load".as_ptr().cast())
    };
    if value.is_null() {
        return Err(BackendEmitError("LLVM failed to load array element".to_owned()));
    }
    values.insert(*result, value);
    Ok(false)
}

pub(super) fn emit_array_element_store(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    let BackendInstruction::ArrayElementStore {
        base,
        index,
        element_layout,
        element_type,
        value,
        ..
    } = instruction
    else {
        return Err(BackendEmitError("LLVM array element store expected".to_owned()));
    };
    let base = values
        .get(base)
        .copied()
        .ok_or_else(|| BackendEmitError(format!("LLVM array base {} is unavailable", base.0)))?;
    let index = values
        .get(index)
        .copied()
        .ok_or_else(|| BackendEmitError(format!("LLVM array index {} is unavailable", index.0)))?;
    let value = values
        .get(value)
        .copied()
        .ok_or_else(|| BackendEmitError(format!("LLVM array value {} is unavailable", value.0)))?;
    let address = array_element_address(context, builder, base, index, *element_layout)?;
    store_array_element_value(context, builder, address, *element_layout, *element_type, value)?;
    Ok(false)
}

fn store_array_element_value(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    address: LLVMValueRef,
    element_layout: super::super::BackendLayout,
    element_type: BackendValueType,
    value: LLVMValueRef,
) -> Result<(), BackendEmitError> {
    if matches!(element_type, BackendValueType::Aggregate { .. } | BackendValueType::Region { .. })
    {
        let size = unsafe {
            LLVMConstInt(LLVMInt64TypeInContext(context), u64::from(element_layout.size), 0)
        };
        if unsafe {
            LLVMBuildMemMove(
                builder,
                address,
                element_layout.alignment,
                value,
                element_layout.alignment,
                size,
            )
        }
        .is_null()
        {
            return Err(BackendEmitError("LLVM failed to copy array aggregate element".to_owned()));
        }
    } else {
        let value_type = super::values::llvm_type(context, element_type)?;
        if unsafe { LLVMBuildStore(builder, value, address) }.is_null() {
            return Err(BackendEmitError("LLVM failed to store array element".to_owned()));
        }
        let _ = value_type;
    }
    Ok(())
}

fn array_element_address(
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    base: LLVMValueRef,
    index: LLVMValueRef,
    element_layout: super::super::BackendLayout,
) -> Result<LLVMValueRef, BackendEmitError> {
    let index_type = unsafe { LLVMInt64TypeInContext(context) };
    let index =
        unsafe { LLVMBuildZExt(builder, index, index_type, c"array.index".as_ptr().cast()) };
    let scale = unsafe { LLVMConstInt(index_type, u64::from(element_layout.size), 0) };
    let offset = unsafe { LLVMBuildMul(builder, index, scale, c"array.offset".as_ptr().cast()) };
    let address = unsafe {
        LLVMBuildGEP2(
            builder,
            LLVMInt8TypeInContext(context),
            base,
            &offset as *const _ as *mut _,
            1,
            c"array.element".as_ptr().cast(),
        )
    };
    (!address.is_null())
        .then_some(address)
        .ok_or_else(|| BackendEmitError("LLVM failed to address array element".to_owned()))
}
