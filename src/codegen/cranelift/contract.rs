use std::collections::BTreeMap;

use cranelift_codegen::ir::{AbiParam, InstBuilder, Type, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module};
use cranelift_object::ObjectModule;

use super::native::BackendEmitError;
use super::object::create_module;
use crate::codegen::{
    BackendFunction, BackendInstruction, BackendModule, BackendPassingMode, BackendValueId,
    BackendValueType,
};
use crate::configuration::NativeBackendConfiguration;
use crate::target::TargetSpec;

pub fn emit_contract_module_object(
    backend_module: &BackendModule,
    configuration: &NativeBackendConfiguration,
    target: &TargetSpec,
) -> Result<Vec<u8>, BackendEmitError> {
    backend_module.validate().map_err(|error| BackendEmitError(error.to_string()))?;
    let mut module = create_module(configuration, target)?;
    let declarations = declare_functions(&mut module, backend_module)?;
    for function in &backend_module.functions {
        define_function(&mut module, function, &declarations)?;
    }
    module.finish().emit().map_err(|error| BackendEmitError(error.to_string()))
}

#[derive(Clone, Copy)]
struct Declaration {
    id: cranelift_module::FuncId,
}

fn declare_functions(
    module: &mut ObjectModule,
    backend_module: &BackendModule,
) -> Result<BTreeMap<String, Declaration>, BackendEmitError> {
    let mut declarations = BTreeMap::new();
    for external in &backend_module.externals {
        let signature = signature(module, &external.parameters, external.return_value.ty)?;
        let id = module
            .declare_function(external.symbol.as_str(), Linkage::Import, &signature)
            .map_err(|error| BackendEmitError(error.to_string()))?;
        declarations.insert(external.symbol.as_str().to_owned(), Declaration { id });
    }
    for function in &backend_module.functions {
        let signature = signature(module, &function.parameters, function.return_value.ty)?;
        let id = module
            .declare_function(function.symbol.as_str(), Linkage::Export, &signature)
            .map_err(|error| BackendEmitError(error.to_string()))?;
        declarations.insert(function.symbol.as_str().to_owned(), Declaration { id });
    }
    Ok(declarations)
}

fn signature(
    module: &ObjectModule,
    parameters: &[crate::codegen::BackendParameter],
    return_value: BackendValueType,
) -> Result<cranelift_codegen::ir::Signature, BackendEmitError> {
    if parameters.iter().any(|parameter| !matches!(parameter.passing, BackendPassingMode::Direct)) {
        return Err(BackendEmitError(
            "Cranelift neutral contract slice supports direct parameters only".to_owned(),
        ));
    }
    let mut signature = module.make_signature();
    for parameter in parameters {
        let Some(ty) = scalar_type(module, parameter.ty)? else {
            return Err(BackendEmitError(
                "Cranelift neutral contract slice supports scalar parameters only".to_owned(),
            ));
        };
        signature.params.push(AbiParam::new(ty));
    }
    if let Some(ty) = scalar_type(module, return_value)? {
        signature.returns.push(AbiParam::new(ty));
    }
    Ok(signature)
}

fn scalar_type(
    module: &ObjectModule,
    ty: BackendValueType,
) -> Result<Option<Type>, BackendEmitError> {
    let result = match ty {
        BackendValueType::Unit => None,
        BackendValueType::Bool => Some(types::I8),
        BackendValueType::Integer { width: 8, .. } => Some(types::I8),
        BackendValueType::Integer { width: 16, .. } => Some(types::I16),
        BackendValueType::Integer { width: 32, .. } => Some(types::I32),
        BackendValueType::Integer { width: 64, .. } => Some(types::I64),
        BackendValueType::Pointer => Some(module.isa().pointer_type()),
        BackendValueType::Integer { .. }
        | BackendValueType::Float { .. }
        | BackendValueType::Aggregate { .. }
        | BackendValueType::Region { .. } => {
            return Err(BackendEmitError(
                "Cranelift neutral contract slice supports scalar values only".to_owned(),
            ));
        }
    };
    Ok(result)
}

fn define_function(
    module: &mut ObjectModule,
    function: &BackendFunction,
    declarations: &BTreeMap<String, Declaration>,
) -> Result<(), BackendEmitError> {
    if function.blocks.len() != 1 {
        return Err(BackendEmitError(
            "Cranelift neutral contract slice supports one block per function".to_owned(),
        ));
    }
    let declaration = declarations
        .get(function.symbol.as_str())
        .copied()
        .ok_or_else(|| BackendEmitError("Cranelift function declaration is missing".to_owned()))?;
    let mut context = module.make_context();
    context.func.signature = signature(module, &function.parameters, function.return_value.ty)?;
    let frontend_config = module.isa().frontend_config();
    let mut function_context = FunctionBuilderContext::new();
    {
        let mut builder = FunctionBuilder::new(&mut context.func, &mut function_context);
        let block = builder.create_block();
        builder.switch_to_block(block);
        builder.append_block_params_for_function_params(block);
        let block_params = builder.block_params(block).to_vec();
        let mut values = BTreeMap::new();
        for (index, value) in block_params.into_iter().enumerate() {
            values.insert(BackendValueId(index as u32), value);
        }
        for instruction in &function.blocks[0].instructions {
            match instruction {
                BackendInstruction::Call { .. } => {
                    lower_call(module, &mut builder, instruction, declarations, &mut values)?;
                }
                BackendInstruction::Cleanup { action, .. }
                    if matches!(
                        action,
                        crate::codegen::LoweredCleanupInstruction::EndBorrow { .. }
                    ) => {}
                BackendInstruction::Return { .. } => {
                    lower_return(&mut builder, instruction, &values)?
                }
                _ => {
                    return Err(BackendEmitError(
                        "Cranelift neutral contract instruction is not implemented".to_owned(),
                    ));
                }
            }
        }
        builder.seal_block(block);
        builder.finalize(frontend_config);
    }
    module
        .define_function(declaration.id, &mut context)
        .map_err(|error| BackendEmitError(error.to_string()))?;
    module.clear_context(&mut context);
    Ok(())
}

fn lower_call(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    declarations: &BTreeMap<String, Declaration>,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::Call { callee, arguments, result, .. } = instruction else {
        return Err(BackendEmitError("Cranelift call instruction expected".to_owned()));
    };
    let declaration = declarations.get(callee.as_str()).ok_or_else(|| {
        BackendEmitError(format!("Cranelift call `{}` is undeclared", callee.as_str()))
    })?;
    let reference = module.declare_func_in_func(declaration.id, builder.func);
    let arguments = arguments
        .iter()
        .map(|value| {
            values.get(value).copied().ok_or_else(|| {
                BackendEmitError(format!("Cranelift call value {} is unavailable", value.0))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let call = builder.ins().call(reference, &arguments);
    if let Some(result) = result {
        let value =
            builder.inst_results(call).first().copied().ok_or_else(|| {
                BackendEmitError("Cranelift call result is unavailable".to_owned())
            })?;
        values.insert(*result, value);
    }
    Ok(())
}

fn lower_return(
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::Return { value, .. } = instruction else {
        return Err(BackendEmitError("Cranelift return instruction expected".to_owned()));
    };
    let value = value
        .map(|value| {
            values.get(&value).copied().ok_or_else(|| {
                BackendEmitError(format!("Cranelift return value {} is unavailable", value.0))
            })
        })
        .transpose()?;
    match value {
        Some(value) => builder.ins().return_(&[value]),
        None => builder.ins().return_(&[]),
    };
    Ok(())
}
