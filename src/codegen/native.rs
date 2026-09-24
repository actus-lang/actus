use std::collections::HashMap;

use cranelift_codegen::ir::{AbiParam, FuncRef, InstBuilder, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module, default_libcall_names};
use cranelift_object::{ObjectBuilder, ObjectModule};

use crate::ast::{ExternalVerbDecl, Program, TopLevelDecl, VerbDecl};
use crate::configuration::NativeBackendConfiguration;
use crate::semantic::{GenericInstance, analyze};

use super::abi::{validate_external_native_signature, validate_native_signature};
use super::declarations::declare_functions;
use super::function_definition::define_function;
use super::generic_layout::GenericLayoutRegistry;
use super::layout::LayoutRegistry;
use super::literals::{StringDataIds, define_string_data};
use super::model::{NativeCleanupSchedule, validate_cleanup_plans};
use super::native_runtime::declare_runtime_functions;
use super::performance::PerformanceRegistry;
use super::performance_emit::define_performances;
use super::target::build_isa;
use super::types::NativeType;
use crate::target::TargetSpec;

#[derive(Debug)]
pub struct NativeEmitError(pub(super) String);

pub(super) struct FunctionMeta {
    pub(super) id: FuncId,
    pub(super) parameter_names: Vec<String>,
    pub(super) return_type: NativeType,
    pub(super) dynamic_params: Vec<bool>,
    pub(super) dynamic_roles: Vec<Option<String>>,
}

pub(super) struct FunctionRef {
    pub(super) reference: FuncRef,
    pub(super) parameter_names: Vec<String>,
    pub(super) return_type: NativeType,
    pub(super) dynamic_params: Vec<bool>,
    pub(super) dynamic_roles: Vec<Option<String>>,
}

impl std::fmt::Display for NativeEmitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for NativeEmitError {}

pub fn emit_zero_return_object(symbol: &str) -> Result<Vec<u8>, NativeEmitError> {
    let target = TargetSpec::host().map_err(|error| NativeEmitError(error.to_string()))?;
    emit_i32_object(symbol, 0, &NativeBackendConfiguration::default(), &target)
}

pub fn emit_program_object(program: &Program, symbol: &str) -> Result<Vec<u8>, NativeEmitError> {
    emit_program_object_with_configuration(program, symbol, &NativeBackendConfiguration::default())
}

pub fn emit_program_object_with_configuration(
    program: &Program,
    symbol: &str,
    configuration: &NativeBackendConfiguration,
) -> Result<Vec<u8>, NativeEmitError> {
    let target = TargetSpec::host().map_err(|error| NativeEmitError(error.to_string()))?;
    emit_program_object_for_target(program, symbol, configuration, &target)
}

pub fn emit_program_object_for_target(
    program: &Program,
    symbol: &str,
    configuration: &NativeBackendConfiguration,
    target: &TargetSpec,
) -> Result<Vec<u8>, NativeEmitError> {
    let semantic = analyze(program)
        .map_err(|error| NativeEmitError(format!("semantic analysis failed: {error:?}")))?;
    validate_cleanup_plans(&semantic)
        .map_err(|error| NativeEmitError(format!("invalid cleanup plan: {error}")))?;
    let cleanup_schedule = NativeCleanupSchedule::from_model(&semantic);
    let performance_registry =
        PerformanceRegistry::from_reachable(&semantic.reachable_performances);
    performance_registry.validate().map_err(NativeEmitError)?;
    let performance_definitions = performance_registry.definitions(program)?;
    let verbs = program
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::Verb(verb) => Some(verb),
            TopLevelDecl::ExternalVerb(_)
            | TopLevelDecl::Struct(_)
            | TopLevelDecl::Enum(_)
            | TopLevelDecl::Role(_)
            | TopLevelDecl::Perform(_) => None,
        })
        .collect::<Vec<_>>();
    let external_verbs = program
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::Verb(_) => None,
            TopLevelDecl::ExternalVerb(verb) => Some(verb),
            TopLevelDecl::Struct(_)
            | TopLevelDecl::Enum(_)
            | TopLevelDecl::Role(_)
            | TopLevelDecl::Perform(_) => None,
        })
        .collect::<Vec<_>>();
    if verbs.is_empty() {
        return Err(NativeEmitError("program has no verb declarations".to_owned()));
    }
    if !verbs.iter().any(|verb| verb.name == symbol) {
        return Err(NativeEmitError(format!("entry verb `{symbol}` was not found")));
    }
    emit_verbs_object(
        program,
        &verbs,
        &external_verbs,
        &semantic.generic_instances,
        &performance_definitions,
        symbol,
        &cleanup_schedule,
        configuration,
        target,
    )
}

#[allow(clippy::too_many_arguments)]
fn emit_verbs_object(
    program: &Program,
    verbs: &[&VerbDecl],
    external_verbs: &[&ExternalVerbDecl],
    generic_instances: &[GenericInstance],
    performance_definitions: &[super::performance::PerformanceDefinition<'_>],
    symbol: &str,
    cleanup_schedule: &NativeCleanupSchedule,
    configuration: &NativeBackendConfiguration,
    target: &TargetSpec,
) -> Result<Vec<u8>, NativeEmitError> {
    let mut module = create_module(configuration, target)?;
    GenericLayoutRegistry::from_program(
        program,
        generic_instances,
        module.isa().pointer_type().bytes(),
    )?;
    let layouts = LayoutRegistry::from_program_with_instances(
        program,
        module.isa().pointer_type(),
        generic_instances,
    )?;
    validate_native_program(verbs, external_verbs, &layouts)?;
    let frontend_config = module.isa().frontend_config();
    let metadata = declare_all_functions(
        &mut module,
        verbs,
        external_verbs,
        performance_definitions,
        symbol,
        &layouts,
    )?;
    let string_data = define_string_data(&mut module, verbs).map_err(NativeEmitError)?;
    let functions = function_metadata(&metadata);
    let vtable_data =
        super::vtable::define_vtables(&mut module, performance_definitions, &functions, &layouts)?;
    define_verbs(
        &mut module,
        frontend_config,
        verbs,
        &functions,
        cleanup_schedule,
        &string_data,
        &layouts,
        &vtable_data,
    )?;
    define_performances(
        &mut module,
        frontend_config,
        performance_definitions,
        &functions,
        cleanup_schedule,
        &string_data,
        &layouts,
        &vtable_data,
    )?;
    module.finish().emit().map_err(|error| NativeEmitError(error.to_string()))
}

fn declare_all_functions(
    module: &mut ObjectModule,
    verbs: &[&VerbDecl],
    external_verbs: &[&ExternalVerbDecl],
    performance_definitions: &[super::performance::PerformanceDefinition<'_>],
    symbol: &str,
    layouts: &LayoutRegistry,
) -> Result<HashMap<String, FunctionMeta>, NativeEmitError> {
    let mut metadata = declare_functions(module, verbs, external_verbs, symbol, layouts)?;
    metadata.extend(super::performance::declare_performance_functions(
        module,
        performance_definitions,
        layouts,
    )?);
    metadata.extend(declare_runtime_functions(module)?);
    Ok(metadata)
}

fn validate_native_program(
    verbs: &[&VerbDecl],
    external_verbs: &[&ExternalVerbDecl],
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    for verb in verbs {
        validate_native_signature(verb, layouts)
            .map_err(|error| NativeEmitError(error.to_string()))?;
    }
    for verb in external_verbs {
        validate_external_native_signature(verb, layouts)
            .map_err(|error| NativeEmitError(error.to_string()))?;
    }
    Ok(())
}

fn function_metadata(metadata: &HashMap<String, FunctionMeta>) -> HashMap<String, FunctionMeta> {
    metadata
        .iter()
        .map(|(name, meta)| {
            (
                name.clone(),
                FunctionMeta {
                    id: meta.id,
                    parameter_names: meta.parameter_names.clone(),
                    return_type: meta.return_type,
                    dynamic_params: meta.dynamic_params.clone(),
                    dynamic_roles: meta.dynamic_roles.clone(),
                },
            )
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn define_verbs(
    module: &mut ObjectModule,
    frontend_config: cranelift_codegen::isa::TargetFrontendConfig,
    verbs: &[&VerbDecl],
    functions: &HashMap<String, FunctionMeta>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataIds,
    layouts: &LayoutRegistry,
    vtable_data: &super::vtable::VtableDataIds,
) -> Result<(), NativeEmitError> {
    for verb in verbs {
        let meta = functions
            .get(&verb.name)
            .ok_or_else(|| NativeEmitError(format!("missing native function `{}`", verb.name)))?;
        define_function(
            module,
            frontend_config,
            verb,
            meta,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
            vtable_data,
        )?;
    }
    Ok(())
}

fn create_module(
    configuration: &NativeBackendConfiguration,
    target: &TargetSpec,
) -> Result<ObjectModule, NativeEmitError> {
    let isa = build_isa(target, configuration.position_independent())
        .map_err(|error| NativeEmitError(error.to_string()))?;
    let builder = ObjectBuilder::new(isa, configuration.module_name(), default_libcall_names())
        .map_err(|error| NativeEmitError(error.to_string()))?;
    Ok(ObjectModule::new(builder))
}

fn emit_i32_object(
    symbol: &str,
    value: i64,
    configuration: &NativeBackendConfiguration,
    target: &TargetSpec,
) -> Result<Vec<u8>, NativeEmitError> {
    let mut module = create_module(configuration, target)?;
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
