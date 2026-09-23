use std::collections::HashMap;

use cranelift_codegen::ir::{AbiParam, FuncRef, InstBuilder, types};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module, default_libcall_names};
use cranelift_native::builder as native_builder;
use cranelift_object::{ObjectBuilder, ObjectModule};

use crate::ast::{ExternalVerbDecl, Program, TopLevelDecl, VerbDecl};
use crate::configuration::NativeBackendConfiguration;
use crate::semantic::analyze;

use super::abi::{validate_external_native_signature, validate_native_signature};
use super::declarations::declare_functions;
use super::layout::LayoutRegistry;
use super::literals::{StringDataIds, declare_string_values, define_string_data};
use super::lowering::lower_body;
use super::model::{NativeCleanupSchedule, validate_cleanup_plans};
use super::native_runtime::declare_runtime_functions;
use super::types::NativeType;

#[derive(Debug)]
pub struct NativeEmitError(pub(super) String);

pub(super) struct FunctionMeta {
    pub(super) id: FuncId,
    pub(super) parameter_names: Vec<String>,
    pub(super) return_type: NativeType,
}

pub(super) struct FunctionRef {
    pub(super) reference: FuncRef,
    pub(super) parameter_names: Vec<String>,
    pub(super) return_type: NativeType,
}

impl std::fmt::Display for NativeEmitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for NativeEmitError {}

pub fn emit_zero_return_object(symbol: &str) -> Result<Vec<u8>, NativeEmitError> {
    emit_i32_object(symbol, 0, &NativeBackendConfiguration::default())
}

pub fn emit_program_object(program: &Program, symbol: &str) -> Result<Vec<u8>, NativeEmitError> {
    emit_program_object_with_configuration(program, symbol, &NativeBackendConfiguration::default())
}

pub fn emit_program_object_with_configuration(
    program: &Program,
    symbol: &str,
    configuration: &NativeBackendConfiguration,
) -> Result<Vec<u8>, NativeEmitError> {
    let semantic = analyze(program)
        .map_err(|error| NativeEmitError(format!("semantic analysis failed: {error:?}")))?;
    validate_cleanup_plans(&semantic)
        .map_err(|error| NativeEmitError(format!("invalid cleanup plan: {error}")))?;
    let cleanup_schedule = NativeCleanupSchedule::from_model(&semantic);
    let verbs = program
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::Verb(verb) => Some(verb),
            TopLevelDecl::ExternalVerb(_) | TopLevelDecl::Struct(_) | TopLevelDecl::Enum(_) => None,
        })
        .collect::<Vec<_>>();
    let external_verbs = program
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::Verb(_) => None,
            TopLevelDecl::ExternalVerb(verb) => Some(verb),
            TopLevelDecl::Struct(_) | TopLevelDecl::Enum(_) => None,
        })
        .collect::<Vec<_>>();
    if verbs.is_empty() {
        return Err(NativeEmitError("program has no verb declarations".to_owned()));
    }
    if !verbs.iter().any(|verb| verb.name == symbol) {
        return Err(NativeEmitError(format!("entry verb `{symbol}` was not found")));
    }
    emit_verbs_object(program, &verbs, &external_verbs, symbol, &cleanup_schedule, configuration)
}

fn emit_verbs_object(
    program: &Program,
    verbs: &[&VerbDecl],
    external_verbs: &[&ExternalVerbDecl],
    symbol: &str,
    cleanup_schedule: &NativeCleanupSchedule,
    configuration: &NativeBackendConfiguration,
) -> Result<Vec<u8>, NativeEmitError> {
    let mut module = create_module(configuration)?;
    let layouts = LayoutRegistry::from_program(program, module.isa().pointer_type())?;
    for verb in verbs {
        validate_native_signature(verb, &layouts)
            .map_err(|error| NativeEmitError(error.to_string()))?;
    }
    for verb in external_verbs {
        validate_external_native_signature(verb, &layouts)
            .map_err(|error| NativeEmitError(error.to_string()))?;
    }
    let frontend_config = module.isa().frontend_config();
    let mut metadata = declare_functions(&mut module, verbs, external_verbs, symbol, &layouts)?;
    let string_data = define_string_data(&mut module, verbs).map_err(NativeEmitError)?;
    metadata.extend(declare_runtime_functions(&mut module)?);
    let functions = metadata
        .iter()
        .map(|(name, meta)| {
            (
                name.clone(),
                FunctionMeta {
                    id: meta.id,
                    parameter_names: meta.parameter_names.clone(),
                    return_type: meta.return_type,
                },
            )
        })
        .collect::<HashMap<_, _>>();
    for verb in verbs {
        let name = verb.name.as_str();
        let meta = functions
            .get(name)
            .ok_or_else(|| NativeEmitError(format!("missing native function `{name}`")))?;
        define_function(
            &mut module,
            frontend_config,
            verb,
            meta,
            &functions,
            cleanup_schedule,
            &string_data,
            &layouts,
        )?;
    }
    module.finish().emit().map_err(|error| NativeEmitError(error.to_string()))
}

fn create_module(
    configuration: &NativeBackendConfiguration,
) -> Result<ObjectModule, NativeEmitError> {
    let mut flag_builder = settings::builder();
    flag_builder
        .set("is_pic", &configuration.position_independent().to_string())
        .map_err(|error| NativeEmitError(error.to_string()))?;
    let flags = settings::Flags::new(flag_builder);
    let isa = native_builder()
        .map_err(|error| NativeEmitError(error.to_string()))?
        .finish(flags)
        .map_err(|error| NativeEmitError(error.to_string()))?;
    let builder = ObjectBuilder::new(isa, configuration.module_name(), default_libcall_names())
        .map_err(|error| NativeEmitError(error.to_string()))?;
    Ok(ObjectModule::new(builder))
}

#[allow(clippy::too_many_arguments)]
fn define_function(
    module: &mut ObjectModule,
    frontend_config: cranelift_codegen::isa::TargetFrontendConfig,
    verb: &VerbDecl,
    metadata: &FunctionMeta,
    functions: &HashMap<String, FunctionMeta>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataIds,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    let mut context = module.make_context();
    context.func.signature =
        super::declarations::native_signature_for_definition(module, verb, layouts);
    let references = declare_function_refs(module, &mut context.func, functions)?;
    let string_values = declare_string_values(module, &mut context.func, string_data);
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
        let local_types = verb
            .params
            .iter()
            .map(|parameter| {
                (
                    &parameter.name,
                    NativeType::from_name_with_layout(&parameter.ty.name, layouts).unwrap(),
                )
            })
            .collect();
        function.seal_block(block);
        let result = lower_body(
            &mut function,
            &verb.body.statements,
            &locals,
            &local_types,
            &references,
            cleanup_schedule,
            &string_values,
            layouts,
        )?;
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
                FunctionRef {
                    reference,
                    parameter_names: meta.parameter_names.clone(),
                    return_type: meta.return_type,
                },
            ))
        })
        .collect()
}

fn emit_i32_object(
    symbol: &str,
    value: i64,
    configuration: &NativeBackendConfiguration,
) -> Result<Vec<u8>, NativeEmitError> {
    let mut flag_builder = settings::builder();
    flag_builder
        .set("is_pic", &configuration.position_independent().to_string())
        .map_err(|error| NativeEmitError(error.to_string()))?;
    let flags = settings::Flags::new(flag_builder);
    let isa = native_builder()
        .map_err(|error| NativeEmitError(error.to_string()))?
        .finish(flags)
        .map_err(|error| NativeEmitError(error.to_string()))?;
    let builder = ObjectBuilder::new(isa, configuration.module_name(), default_libcall_names())
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
