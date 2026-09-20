use std::collections::HashMap;

use cranelift_codegen::ir::{AbiParam, FuncRef, InstBuilder, types};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module, default_libcall_names};
use cranelift_native::builder as native_builder;
use cranelift_object::{ObjectBuilder, ObjectModule};

use crate::ast::{Program, TopLevelDecl, VerbDecl};
use crate::semantic::analyze;

use super::abi::validate_integer_signature;
use super::lowering::lower_body;
use super::model::validate_cleanup_plans;

#[derive(Debug)]
pub struct NativeEmitError(pub(super) String);

struct FunctionMeta {
    id: FuncId,
    parameter_names: Vec<String>,
}

pub(super) struct FunctionRef {
    pub(super) reference: FuncRef,
    pub(super) parameter_names: Vec<String>,
}

impl std::fmt::Display for NativeEmitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for NativeEmitError {}

pub fn emit_zero_return_object(symbol: &str) -> Result<Vec<u8>, NativeEmitError> {
    emit_i32_object(symbol, 0)
}

pub fn emit_program_object(program: &Program, symbol: &str) -> Result<Vec<u8>, NativeEmitError> {
    let semantic = analyze(program)
        .map_err(|error| NativeEmitError(format!("semantic analysis failed: {error:?}")))?;
    validate_cleanup_plans(&semantic)
        .map_err(|error| NativeEmitError(format!("invalid cleanup plan: {error}")))?;
    let verbs = program
        .declarations
        .iter()
        .map(|declaration| match declaration {
            TopLevelDecl::Verb(verb) => verb,
        })
        .collect::<Vec<_>>();
    if verbs.is_empty() {
        return Err(NativeEmitError("program has no verb declarations".to_owned()));
    }
    for verb in &verbs {
        validate_integer_signature(verb).map_err(|error| NativeEmitError(error.to_string()))?;
    }
    emit_verbs_object(&verbs, symbol)
}

fn emit_verbs_object(verbs: &[&VerbDecl], symbol: &str) -> Result<Vec<u8>, NativeEmitError> {
    let mut module = create_module()?;
    let frontend_config = module.isa().frontend_config();
    let metadata = declare_functions(&mut module, verbs, symbol)?;
    let functions = metadata
        .iter()
        .map(|(name, meta)| {
            (
                name.clone(),
                FunctionMeta { id: meta.id, parameter_names: meta.parameter_names.clone() },
            )
        })
        .collect::<HashMap<_, _>>();
    for verb in verbs {
        let name = verb.name.as_str();
        let meta = functions
            .get(name)
            .ok_or_else(|| NativeEmitError(format!("missing native function `{name}`")))?;
        define_function(&mut module, frontend_config, verb, meta, &functions)?;
    }
    module.finish().emit().map_err(|error| NativeEmitError(error.to_string()))
}

fn create_module() -> Result<ObjectModule, NativeEmitError> {
    let mut flag_builder = settings::builder();
    flag_builder.set("is_pic", "true").map_err(|error| NativeEmitError(error.to_string()))?;
    let flags = settings::Flags::new(flag_builder);
    let isa = native_builder()
        .map_err(|error| NativeEmitError(error.to_string()))?
        .finish(flags)
        .map_err(|error| NativeEmitError(error.to_string()))?;
    let builder = ObjectBuilder::new(isa, "actus", default_libcall_names())
        .map_err(|error| NativeEmitError(error.to_string()))?;
    Ok(ObjectModule::new(builder))
}

fn declare_functions(
    module: &mut ObjectModule,
    verbs: &[&VerbDecl],
    entry_symbol: &str,
) -> Result<HashMap<String, FunctionMeta>, NativeEmitError> {
    let mut metadata = HashMap::new();
    for (index, verb) in verbs.iter().enumerate() {
        let signature = integer_signature(module, verb);
        let symbol = if index == 0 { entry_symbol } else { &verb.name };
        let id = module
            .declare_function(symbol, Linkage::Export, &signature)
            .map_err(|error| NativeEmitError(error.to_string()))?;
        metadata.insert(
            verb.name.clone(),
            FunctionMeta {
                id,
                parameter_names: verb.params.iter().map(|param| param.name.clone()).collect(),
            },
        );
    }
    Ok(metadata)
}

fn integer_signature(
    module: &mut ObjectModule,
    verb: &VerbDecl,
) -> cranelift_codegen::ir::Signature {
    let mut signature = module.make_signature();
    signature.params.extend((0..verb.params.len()).map(|_| AbiParam::new(types::I32)));
    signature.returns.push(AbiParam::new(types::I32));
    signature
}

fn define_function(
    module: &mut ObjectModule,
    frontend_config: cranelift_codegen::isa::TargetFrontendConfig,
    verb: &VerbDecl,
    metadata: &FunctionMeta,
    functions: &HashMap<String, FunctionMeta>,
) -> Result<(), NativeEmitError> {
    let mut context = module.make_context();
    context.func.signature = integer_signature(module, verb);
    let references = declare_function_refs(module, &mut context.func, functions)?;
    let mut function_context = FunctionBuilderContext::new();
    {
        let mut function = FunctionBuilder::new(&mut context.func, &mut function_context);
        let block = function.create_block();
        function.switch_to_block(block);
        function.append_block_params_for_function_params(block);
        let parameters = function.block_params(block).to_vec();
        let mut locals = HashMap::new();
        for (parameter, value) in verb.params.iter().zip(parameters) {
            locals.insert(&parameter.name, value);
        }
        function.seal_block(block);
        let result = lower_body(&mut function, &verb.body.statements, &locals, &references)?;
        function.ins().return_(&[result]);
        function.finalize(frontend_config);
    }
    module
        .define_function(metadata.id, &mut context)
        .map_err(|error| NativeEmitError(error.to_string()))?;
    module.clear_context(&mut context);
    Ok(())
}

fn declare_function_refs(
    module: &mut ObjectModule,
    function: &mut cranelift_codegen::ir::Function,
    functions: &HashMap<String, FunctionMeta>,
) -> Result<HashMap<String, FunctionRef>, NativeEmitError> {
    functions
        .iter()
        .map(|(name, meta)| {
            let reference = module.declare_func_in_func(meta.id, function);
            Ok((
                name.clone(),
                FunctionRef { reference, parameter_names: meta.parameter_names.clone() },
            ))
        })
        .collect()
}

fn emit_i32_object(symbol: &str, value: i64) -> Result<Vec<u8>, NativeEmitError> {
    let mut flag_builder = settings::builder();
    flag_builder.set("is_pic", "true").map_err(|error| NativeEmitError(error.to_string()))?;
    let flags = settings::Flags::new(flag_builder);
    let isa = native_builder()
        .map_err(|error| NativeEmitError(error.to_string()))?
        .finish(flags)
        .map_err(|error| NativeEmitError(error.to_string()))?;
    let builder = ObjectBuilder::new(isa, "actus", default_libcall_names())
        .map_err(|error| NativeEmitError(error.to_string()))?;
    let mut module = ObjectModule::new(builder);
    let frontend_config = module.isa().frontend_config();
    let mut signature = module.make_signature();
    signature.returns.push(AbiParam::new(types::I32));
    let function_id = module
        .declare_function(symbol, Linkage::Export, &signature)
        .map_err(|error| NativeEmitError(error.to_string()))?;
    let mut context = module.make_context();
    context.func.signature = signature;
    let mut function_context = FunctionBuilderContext::new();
    {
        let mut function = FunctionBuilder::new(&mut context.func, &mut function_context);
        let block = function.create_block();
        function.switch_to_block(block);
        function.seal_block(block);
        let result = function.ins().iconst(types::I32, value);
        function.ins().return_(&[result]);
        function.finalize(frontend_config);
    }
    module
        .define_function(function_id, &mut context)
        .map_err(|error| NativeEmitError(error.to_string()))?;
    module.clear_context(&mut context);
    module.finish().emit().map_err(|error| NativeEmitError(error.to_string()))
}
