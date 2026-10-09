use std::collections::BTreeMap;
use std::ffi::{CStr, CString};

use llvm_sys::core::{
    LLVMAddFunction, LLVMAppendBasicBlockInContext, LLVMBuildRetVoid, LLVMContextCreate,
    LLVMContextDispose, LLVMCreateBuilderInContext, LLVMDisposeBuilder, LLVMDisposeMessage,
    LLVMDisposeModule, LLVMFunctionType, LLVMInt1TypeInContext, LLVMInt8TypeInContext,
    LLVMInt16TypeInContext, LLVMInt32TypeInContext, LLVMInt64TypeInContext,
    LLVMModuleCreateWithNameInContext, LLVMPrintModuleToString, LLVMVoidTypeInContext,
};
use llvm_sys::prelude::{LLVMBuilderRef, LLVMContextRef, LLVMModuleRef, LLVMTypeRef};

use super::{
    BackendBlockId, BackendEmitError, BackendFunction, BackendInstruction, BackendModule,
    BackendReturnMode, BackendValueType,
};

/// Emits the supported first slice of the Actus backend-neutral contract as LLVM IR.
///
/// The initial slice covers validated scalar and unit function signatures, basic blocks,
/// and direct returns. Aggregate values, calls, cleanup instructions, and typed failure
/// lowering remain explicit unsupported operations until their corresponding gates are added.
pub fn emit_module(module: &BackendModule) -> Result<String, BackendEmitError> {
    module.validate().map_err(|error| BackendEmitError(error.to_string()))?;
    let context = LlvmContext::new()?;
    let module_name = CString::new("actus.module")
        .map_err(|_| BackendEmitError("LLVM module name contains NUL".to_owned()))?;
    let llvm_module =
        unsafe { LLVMModuleCreateWithNameInContext(module_name.as_ptr(), context.raw) };
    if llvm_module.is_null() {
        return Err(BackendEmitError("LLVM failed to create a module".to_owned()));
    }
    let owned_module = LlvmModule { raw: llvm_module };
    for function in &module.functions {
        emit_function(context.raw, owned_module.raw, function)?;
    }
    print_module(owned_module.raw)
}

struct LlvmContext {
    raw: LLVMContextRef,
}

impl LlvmContext {
    fn new() -> Result<Self, BackendEmitError> {
        let raw = unsafe { LLVMContextCreate() };
        (!raw.is_null())
            .then_some(Self { raw })
            .ok_or_else(|| BackendEmitError("LLVM failed to create a context".to_owned()))
    }
}

impl Drop for LlvmContext {
    fn drop(&mut self) {
        unsafe { LLVMContextDispose(self.raw) }
    }
}

struct LlvmModule {
    raw: LLVMModuleRef,
}

impl Drop for LlvmModule {
    fn drop(&mut self) {
        unsafe { LLVMDisposeModule(self.raw) }
    }
}

fn emit_function(
    context: LLVMContextRef,
    module: LLVMModuleRef,
    function: &BackendFunction,
) -> Result<(), BackendEmitError> {
    let return_type = llvm_type(context, function.return_value.ty)?;
    if !matches!(function.return_value.passing, BackendReturnMode::Direct) {
        return Err(unsupported(function, "aggregate return slots are not implemented"));
    }
    let mut parameter_types = function
        .parameters
        .iter()
        .map(|parameter| llvm_type(context, parameter.ty))
        .collect::<Result<Vec<_>, _>>()?;
    let function_name = CString::new(function.symbol.as_str())
        .map_err(|_| BackendEmitError("LLVM function symbol contains NUL".to_owned()))?;
    let function_type = unsafe {
        LLVMFunctionType(return_type, parameter_types.as_mut_ptr(), parameter_types.len() as u32, 0)
    };
    let llvm_function = unsafe { LLVMAddFunction(module, function_name.as_ptr(), function_type) };
    if llvm_function.is_null() {
        return Err(BackendEmitError(format!(
            "LLVM rejected function `{}`",
            function.symbol.as_str()
        )));
    }
    let mut blocks = BTreeMap::new();
    for block in &function.blocks {
        let block_name = CString::new(format!("bb{}", block.id.0))
            .map_err(|_| BackendEmitError("LLVM block name contains NUL".to_owned()))?;
        let llvm_block =
            unsafe { LLVMAppendBasicBlockInContext(context, llvm_function, block_name.as_ptr()) };
        blocks.insert(block.id, llvm_block);
    }
    let builder = unsafe { LLVMCreateBuilderInContext(context) };
    if builder.is_null() {
        return Err(BackendEmitError("LLVM failed to create a builder".to_owned()));
    }
    let result = emit_blocks(builder, function, &blocks);
    unsafe { LLVMDisposeBuilder(builder) };
    result
}

fn emit_blocks(
    builder: LLVMBuilderRef,
    function: &BackendFunction,
    blocks: &BTreeMap<BackendBlockId, llvm_sys::prelude::LLVMBasicBlockRef>,
) -> Result<(), BackendEmitError> {
    for block in &function.blocks {
        let llvm_block = blocks
            .get(&block.id)
            .copied()
            .ok_or_else(|| BackendEmitError("LLVM block mapping is incomplete".to_owned()))?;
        unsafe { llvm_sys::core::LLVMPositionBuilderAtEnd(builder, llvm_block) };
        let mut terminated = false;
        for instruction in &block.instructions {
            match instruction {
                BackendInstruction::Return { value: None, .. } => {
                    if !matches!(function.return_value.ty, BackendValueType::Unit) {
                        return Err(BackendEmitError(
                            "non-unit LLVM function requires a return value".to_owned(),
                        ));
                    }
                    unsafe { LLVMBuildRetVoid(builder) };
                    terminated = true;
                }
                BackendInstruction::Return { value: Some(_), .. } => {
                    return Err(BackendEmitError(
                        "scalar LLVM return values are not implemented".to_owned(),
                    ));
                }
                BackendInstruction::Call { .. }
                | BackendInstruction::Branch { .. }
                | BackendInstruction::Jump { .. }
                | BackendInstruction::Fail { .. }
                | BackendInstruction::Trap { .. }
                | BackendInstruction::Cleanup { .. } => {
                    return Err(BackendEmitError(
                        "LLVM instruction lowering is not implemented for this contract node"
                            .to_owned(),
                    ));
                }
            }
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

fn llvm_type(
    context: LLVMContextRef,
    ty: BackendValueType,
) -> Result<LLVMTypeRef, BackendEmitError> {
    let result = match ty {
        BackendValueType::Unit => unsafe { LLVMVoidTypeInContext(context) },
        BackendValueType::Bool => unsafe { LLVMInt1TypeInContext(context) },
        BackendValueType::Integer { width: 8, .. } => unsafe { LLVMInt8TypeInContext(context) },
        BackendValueType::Integer { width: 16, .. } => unsafe { LLVMInt16TypeInContext(context) },
        BackendValueType::Integer { width: 32, .. } => unsafe { LLVMInt32TypeInContext(context) },
        BackendValueType::Integer { width: 64, .. } => unsafe { LLVMInt64TypeInContext(context) },
        BackendValueType::Integer { .. }
        | BackendValueType::Float { .. }
        | BackendValueType::Pointer
        | BackendValueType::Aggregate { .. }
        | BackendValueType::Region { .. } => {
            return Err(BackendEmitError("LLVM type lowering is not implemented".to_owned()));
        }
    };
    Ok(result)
}

fn print_module(module: LLVMModuleRef) -> Result<String, BackendEmitError> {
    let printed = unsafe { LLVMPrintModuleToString(module) };
    if printed.is_null() {
        return Err(BackendEmitError("LLVM failed to print the module".to_owned()));
    }
    let text = unsafe { CStr::from_ptr(printed) }
        .to_str()
        .map_err(|_| BackendEmitError("LLVM produced non-UTF-8 IR".to_owned()))?
        .to_owned();
    unsafe { LLVMDisposeMessage(printed) };
    Ok(text)
}

fn unsupported(function: &BackendFunction, reason: &str) -> BackendEmitError {
    BackendEmitError(format!("LLVM function `{}` unsupported: {reason}", function.symbol.as_str()))
}
