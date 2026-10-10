use std::collections::BTreeMap;
use std::ffi::CString;

use llvm_sys::core::{
    LLVMAppendBasicBlockInContext, LLVMCreateBuilderInContext, LLVMDisposeBuilder,
};
use llvm_sys::prelude::{LLVMBasicBlockRef, LLVMContextRef, LLVMModuleRef, LLVMValueRef};

use super::super::{
    BackendBlockId, BackendEmitError, BackendFunction, BackendModule, BackendValueType,
};
use super::context::{LlvmContext, LlvmModule};
use super::control_flow::emit_blocks;
use super::declarations::{FunctionDeclaration, declare_cleanup_bridge, declare_function};
use crate::codegen::{BackendCleanupBridge, BackendInstruction};
pub(super) fn lower_module(
    module: &BackendModule,
    pointer_width: u8,
) -> Result<(LlvmContext, LlvmModule), BackendEmitError> {
    module.validate().map_err(|error| BackendEmitError(error.to_string()))?;
    validate_integer_only(module)?;
    let context = LlvmContext::new()?;
    let module_name = CString::new("actus.module")
        .map_err(|_| BackendEmitError("LLVM module name contains NUL".to_owned()))?;
    let owned_module = LlvmModule::new(context.raw, module_name.as_ptr())?;
    let declarations =
        declare_module_functions(context.raw, owned_module.raw, module, pointer_width)?;
    for function in &module.functions {
        let declaration = declarations
            .get(function.symbol.as_str())
            .copied()
            .ok_or_else(|| BackendEmitError("LLVM function declaration is missing".to_owned()))?;
        emit_function(context.raw, pointer_width, function, declaration, &declarations)?;
    }
    Ok((context, owned_module))
}

fn validate_integer_only(module: &BackendModule) -> Result<(), BackendEmitError> {
    for function in &module.functions {
        reject_float(function.symbol.as_str(), function.return_value.ty)?;
        for parameter in &function.parameters {
            reject_float(function.symbol.as_str(), parameter.ty)?;
        }
    }
    for external in &module.externals {
        reject_float(external.symbol.as_str(), external.return_value.ty)?;
        for parameter in &external.parameters {
            reject_float(external.symbol.as_str(), parameter.ty)?;
        }
    }
    Ok(())
}

fn reject_float(symbol: &str, ty: BackendValueType) -> Result<(), BackendEmitError> {
    if matches!(ty, BackendValueType::Float { .. }) {
        return Err(BackendEmitError(format!(
            "LLVM integer-only emission rejects floating-point type in `{symbol}`"
        )));
    }
    Ok(())
}

fn declare_module_functions(
    context: LLVMContextRef,
    module: LLVMModuleRef,
    backend_module: &BackendModule,
    pointer_width: u8,
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
    for bridge in cleanup_bridges(backend_module) {
        let symbol = cleanup_bridge_symbol(bridge);
        if declarations.contains_key(symbol) {
            continue;
        }
        let declaration = declare_cleanup_bridge(context, module, bridge, pointer_width)?;
        declarations.insert(symbol.to_owned(), declaration);
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

fn cleanup_bridges(module: &BackendModule) -> Vec<BackendCleanupBridge> {
    let mut bridges = Vec::new();
    for function in &module.functions {
        for block in &function.blocks {
            for instruction in &block.instructions {
                let BackendInstruction::Cleanup { operand: Some(operand), .. } = instruction else {
                    continue;
                };
                if !bridges.contains(&operand.bridge) {
                    bridges.push(operand.bridge);
                }
            }
        }
    }
    bridges
}

fn cleanup_bridge_symbol(bridge: BackendCleanupBridge) -> &'static str {
    match bridge {
        BackendCleanupBridge::BufferDrop => crate::runtime::BUFFER_DROP_SYMBOL,
        BackendCleanupBridge::RegionDrop => crate::runtime::REGION_DROP_SYMBOL,
        BackendCleanupBridge::EnumDrop { .. } => crate::runtime::ENUM_DROP_SYMBOL,
    }
}

fn emit_function(
    context: LLVMContextRef,
    pointer_width: u8,
    function: &BackendFunction,
    declaration: FunctionDeclaration,
    declarations: &BTreeMap<String, FunctionDeclaration>,
) -> Result<(), BackendEmitError> {
    let blocks = append_blocks(context, declaration.value, function)?;
    let builder = unsafe { LLVMCreateBuilderInContext(context) };
    if builder.is_null() {
        return Err(BackendEmitError("LLVM failed to create a builder".to_owned()));
    }
    let result =
        emit_blocks(context, builder, pointer_width, function, declaration, declarations, &blocks);
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
