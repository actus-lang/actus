use std::collections::BTreeMap;
use std::ffi::{CStr, CString};

use llvm_sys::core::{
    LLVMAddFunction, LLVMAppendBasicBlockInContext, LLVMBuildCall2, LLVMBuildRet, LLVMBuildRetVoid,
    LLVMContextCreate, LLVMContextDispose, LLVMCreateBuilderInContext, LLVMDisposeBuilder,
    LLVMDisposeMessage, LLVMDisposeModule, LLVMFunctionType, LLVMGetParam, LLVMInt1TypeInContext,
    LLVMInt8TypeInContext, LLVMInt16TypeInContext, LLVMInt32TypeInContext, LLVMInt64TypeInContext,
    LLVMModuleCreateWithNameInContext, LLVMPrintModuleToString, LLVMVoidTypeInContext,
};
use llvm_sys::prelude::{
    LLVMBasicBlockRef, LLVMBuilderRef, LLVMContextRef, LLVMModuleRef, LLVMTypeRef, LLVMValueRef,
};

use super::{
    BackendBlockId, BackendEmitError, BackendFunction, BackendInstruction, BackendModule,
    BackendParameter, BackendReturn, BackendReturnMode, BackendValueId, BackendValueType,
};

/// Emits the supported first slice of the Actus backend-neutral contract as LLVM IR.
///
/// The initial slice covers validated scalar and unit function signatures, parameter mapping,
/// basic blocks, direct returns, and direct calls. Aggregate values, control flow, cleanup
/// instructions, and typed failure lowering remain explicit unsupported operations until their
/// corresponding gates are added.
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
    let mut declarations = BTreeMap::new();
    for external in &module.externals {
        let declaration = declare_function(
            context.raw,
            owned_module.raw,
            external.symbol.as_str(),
            &external.parameters,
            &external.return_value,
        )?;
        declarations.insert(external.symbol.as_str().to_owned(), declaration);
    }
    for function in &module.functions {
        let declaration = declare_function(
            context.raw,
            owned_module.raw,
            function.symbol.as_str(),
            &function.parameters,
            &function.return_value,
        )?;
        declarations.insert(function.symbol.as_str().to_owned(), declaration);
    }
    for function in &module.functions {
        let declaration = declarations
            .get(function.symbol.as_str())
            .copied()
            .ok_or_else(|| BackendEmitError("LLVM function declaration is missing".to_owned()))?;
        emit_function(context.raw, function, declaration, &declarations)?;
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
    function: &BackendFunction,
    declaration: FunctionDeclaration,
    declarations: &BTreeMap<String, FunctionDeclaration>,
) -> Result<(), BackendEmitError> {
    let llvm_function = declaration.value;
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
    let result = emit_blocks(builder, function, declaration, declarations, &blocks);
    unsafe { LLVMDisposeBuilder(builder) };
    result
}

#[derive(Clone, Copy)]
struct FunctionDeclaration {
    value: LLVMValueRef,
    function_type: LLVMTypeRef,
    parameter_count: usize,
    returns_void: bool,
}

fn declare_function(
    context: LLVMContextRef,
    module: LLVMModuleRef,
    symbol: &str,
    parameters: &[BackendParameter],
    return_value: &BackendReturn,
) -> Result<FunctionDeclaration, BackendEmitError> {
    if !matches!(return_value.passing, BackendReturnMode::Direct) {
        return Err(BackendEmitError("LLVM aggregate return slots are not implemented".to_owned()));
    }
    let return_type = llvm_type(context, return_value.ty)?;
    let mut parameter_types = parameters
        .iter()
        .map(|parameter| llvm_type(context, parameter.ty))
        .collect::<Result<Vec<_>, _>>()?;
    let function_type = unsafe {
        LLVMFunctionType(return_type, parameter_types.as_mut_ptr(), parameter_types.len() as u32, 0)
    };
    let function_name = CString::new(symbol)
        .map_err(|_| BackendEmitError("LLVM function symbol contains NUL".to_owned()))?;
    let value = unsafe { LLVMAddFunction(module, function_name.as_ptr(), function_type) };
    if value.is_null() {
        return Err(BackendEmitError(format!("LLVM rejected function `{symbol}`")));
    }
    Ok(FunctionDeclaration {
        value,
        function_type,
        parameter_count: parameters.len(),
        returns_void: matches!(return_value.ty, BackendValueType::Unit),
    })
}

fn emit_blocks(
    builder: LLVMBuilderRef,
    function: &BackendFunction,
    declaration: FunctionDeclaration,
    declarations: &BTreeMap<String, FunctionDeclaration>,
    blocks: &BTreeMap<BackendBlockId, LLVMBasicBlockRef>,
) -> Result<(), BackendEmitError> {
    let mut values = BTreeMap::new();
    for index in 0..declaration.parameter_count {
        let value = unsafe { LLVMGetParam(declaration.value, index as u32) };
        values.insert(BackendValueId(index as u32), value);
    }
    for block in &function.blocks {
        let llvm_block = blocks
            .get(&block.id)
            .copied()
            .ok_or_else(|| BackendEmitError("LLVM block mapping is incomplete".to_owned()))?;
        unsafe { llvm_sys::core::LLVMPositionBuilderAtEnd(builder, llvm_block) };
        let mut terminated = false;
        for instruction in &block.instructions {
            if terminated {
                return Err(BackendEmitError(
                    "LLVM block has instructions after a terminator".to_owned(),
                ));
            }
            terminated =
                emit_instruction(builder, function, instruction, declarations, &mut values)?;
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

fn emit_instruction(
    builder: LLVMBuilderRef,
    function: &BackendFunction,
    instruction: &BackendInstruction,
    declarations: &BTreeMap<String, FunctionDeclaration>,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<bool, BackendEmitError> {
    match instruction {
        BackendInstruction::Return { value, .. } => {
            emit_return(builder, function, *value, values)?;
            Ok(true)
        }
        BackendInstruction::Call { callee, arguments, result, .. } => {
            emit_call(builder, callee.as_str(), arguments, *result, declarations, values)?;
            Ok(false)
        }
        BackendInstruction::Branch { .. }
        | BackendInstruction::Jump { .. }
        | BackendInstruction::Fail { .. }
        | BackendInstruction::Trap { .. }
        | BackendInstruction::Cleanup { .. } => Err(BackendEmitError(
            "LLVM instruction lowering is not implemented for this contract node".to_owned(),
        )),
    }
}

fn emit_return(
    builder: LLVMBuilderRef,
    function: &BackendFunction,
    value_id: Option<BackendValueId>,
    values: &BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<(), BackendEmitError> {
    match value_id {
        None if matches!(function.return_value.ty, BackendValueType::Unit) => {
            unsafe { LLVMBuildRetVoid(builder) };
            Ok(())
        }
        None => Err(BackendEmitError("non-unit LLVM function requires a return value".to_owned())),
        Some(value_id) => {
            let value = values.get(&value_id).copied().ok_or_else(|| {
                BackendEmitError(format!("LLVM return value {} is unavailable", value_id.0))
            })?;
            unsafe { LLVMBuildRet(builder, value) };
            Ok(())
        }
    }
}

fn emit_call(
    builder: LLVMBuilderRef,
    callee: &str,
    argument_ids: &[BackendValueId],
    result: Option<BackendValueId>,
    declarations: &BTreeMap<String, FunctionDeclaration>,
    values: &mut BTreeMap<BackendValueId, LLVMValueRef>,
) -> Result<(), BackendEmitError> {
    let declaration = declarations
        .get(callee)
        .copied()
        .ok_or_else(|| BackendEmitError(format!("LLVM callee `{callee}` is unavailable")))?;
    if argument_ids.len() != declaration.parameter_count {
        return Err(BackendEmitError(format!(
            "LLVM call `{callee}` expects {} arguments, got {}",
            declaration.parameter_count,
            argument_ids.len()
        )));
    }
    let mut arguments = argument_ids
        .iter()
        .map(|id| {
            values
                .get(id)
                .copied()
                .ok_or_else(|| BackendEmitError(format!("LLVM call value {} is unavailable", id.0)))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let call_name = CString::new(result.map(|id| format!("call{}", id.0)).unwrap_or_default())
        .map_err(|_| BackendEmitError("LLVM call name contains NUL".to_owned()))?;
    let call = unsafe {
        LLVMBuildCall2(
            builder,
            declaration.function_type,
            declaration.value,
            arguments.as_mut_ptr(),
            arguments.len() as u32,
            call_name.as_ptr(),
        )
    };
    if call.is_null() {
        return Err(BackendEmitError("LLVM failed to build call".to_owned()));
    }
    if let Some(result) = result {
        if declaration.returns_void {
            return Err(BackendEmitError("LLVM call result cannot be void".to_owned()));
        }
        values.insert(result, call);
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
