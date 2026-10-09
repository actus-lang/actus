use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::ptr;

use llvm_sys::analysis::{LLVMVerifierFailureAction, LLVMVerifyModule};
use llvm_sys::core::{
    LLVMAppendBasicBlockInContext, LLVMCreateBuilderInContext, LLVMDisposeBuilder,
    LLVMDisposeMessage, LLVMPrintModuleToString,
};
use llvm_sys::prelude::{LLVMBasicBlockRef, LLVMContextRef, LLVMModuleRef, LLVMValueRef};

use super::super::{BackendBlockId, BackendEmitError, BackendFunction, BackendModule};
use super::context::{LlvmContext, LlvmModule};
use super::control_flow::emit_blocks;
use super::declarations::{FunctionDeclaration, declare_function};

/// Emits the supported Actus backend-neutral contract slice as LLVM IR.
///
/// The current slice covers validated scalar and unit signatures, parameter mapping, basic
/// blocks, direct returns, direct calls, conditional branches, jumps, terminating traps, and
/// compile-time end-borrow cleanup boundaries. Runtime-backed cleanup actions and typed failure
/// lowering remain explicit open work.
pub fn emit_module(module: &BackendModule) -> Result<String, BackendEmitError> {
    module.validate().map_err(|error| BackendEmitError(error.to_string()))?;
    let context = LlvmContext::new()?;
    let module_name = CString::new("actus.module")
        .map_err(|_| BackendEmitError("LLVM module name contains NUL".to_owned()))?;
    let owned_module = LlvmModule::new(context.raw, module_name.as_ptr())?;
    let declarations = declare_module_functions(context.raw, owned_module.raw, module)?;
    for function in &module.functions {
        let declaration = declarations
            .get(function.symbol.as_str())
            .copied()
            .ok_or_else(|| BackendEmitError("LLVM function declaration is missing".to_owned()))?;
        emit_function(context.raw, function, declaration, &declarations)?;
    }
    verify_module(owned_module.raw)?;
    print_module(owned_module.raw)
}

fn declare_module_functions(
    context: LLVMContextRef,
    module: LLVMModuleRef,
    backend_module: &BackendModule,
) -> Result<BTreeMap<String, FunctionDeclaration>, BackendEmitError> {
    let mut declarations = BTreeMap::new();
    for external in &backend_module.externals {
        let declaration = declare_function(
            context,
            module,
            external.symbol.as_str(),
            &external.parameters,
            &external.return_value,
        )?;
        declarations.insert(external.symbol.as_str().to_owned(), declaration);
    }
    for function in &backend_module.functions {
        let declaration = declare_function(
            context,
            module,
            function.symbol.as_str(),
            &function.parameters,
            &function.return_value,
        )?;
        declarations.insert(function.symbol.as_str().to_owned(), declaration);
    }
    Ok(declarations)
}

fn emit_function(
    context: LLVMContextRef,
    function: &BackendFunction,
    declaration: FunctionDeclaration,
    declarations: &BTreeMap<String, FunctionDeclaration>,
) -> Result<(), BackendEmitError> {
    let blocks = append_blocks(context, declaration.value, function)?;
    let builder = unsafe { LLVMCreateBuilderInContext(context) };
    if builder.is_null() {
        return Err(BackendEmitError("LLVM failed to create a builder".to_owned()));
    }
    let result = emit_blocks(context, builder, function, declaration, declarations, &blocks);
    unsafe { LLVMDisposeBuilder(builder) };
    result
}

fn append_blocks(
    context: LLVMContextRef,
    function: LLVMValueRef,
    backend_function: &BackendFunction,
) -> Result<BTreeMap<BackendBlockId, LLVMBasicBlockRef>, BackendEmitError> {
    let mut blocks = BTreeMap::new();
    for block in &backend_function.blocks {
        let block_name = CString::new(format!("bb{}", block.id.0))
            .map_err(|_| BackendEmitError("LLVM block name contains NUL".to_owned()))?;
        let llvm_block =
            unsafe { LLVMAppendBasicBlockInContext(context, function, block_name.as_ptr()) };
        blocks.insert(block.id, llvm_block);
    }
    Ok(blocks)
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

fn verify_module(module: LLVMModuleRef) -> Result<(), BackendEmitError> {
    let mut message = ptr::null_mut();
    let failed = unsafe {
        LLVMVerifyModule(module, LLVMVerifierFailureAction::LLVMReturnStatusAction, &mut message)
            != 0
    };
    if !failed {
        return Ok(());
    }
    let diagnostic = if message.is_null() {
        "LLVM module verification failed".to_owned()
    } else {
        let text = unsafe { CStr::from_ptr(message) }.to_string_lossy().into_owned();
        unsafe { LLVMDisposeMessage(message) };
        text
    };
    Err(BackendEmitError(format!("LLVM module verification failed: {diagnostic}")))
}
