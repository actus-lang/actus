use crate::ast::{ExternalVerbDecl, Program, VerbDecl};
use crate::configuration::NativeBackendConfiguration;
use crate::semantic::{GenericInstance, SemanticModel, analyze, filter_program_for_target};
use crate::target::TargetSpec;

use cranelift_module::Module;

use super::super::generic::GenericLayoutRegistry;
use super::super::layout::LayoutRegistry;
use super::super::literals::define_string_data;
use super::super::model::{NativeCleanupSchedule, validate_cleanup_plans};
use super::super::performance::PerformanceRegistry;
use super::super::performance::define_performances;
use super::super::result_constructors::normalize_program;
use super::NativeEmitError;
use super::declarations::{
    declaration_external_verb, declaration_verb, declare_all_functions, define_verbs,
    function_metadata, validate_native_program,
};
use super::object::create_module;

pub(super) fn emit_program_object_for_target(
    program: &Program,
    symbol: &str,
    configuration: &NativeBackendConfiguration,
    target: &TargetSpec,
) -> Result<Vec<u8>, NativeEmitError> {
    let (program, semantic) = prepare_program(program, target)?;
    let cleanup_schedule = NativeCleanupSchedule::from_model(&semantic);
    let performance_registry =
        PerformanceRegistry::from_program(&program, &semantic.reachable_performances);
    performance_registry.validate().map_err(NativeEmitError)?;
    let performance_definitions = performance_registry.definitions(&program)?;
    let (verbs, external_verbs) = collect_declarations(&program);
    validate_entry_verb(&verbs, symbol)?;
    emit_verbs_object(VerbEmission {
        program: &program,
        verbs: &verbs,
        external_verbs: &external_verbs,
        generic_instances: &semantic.generic_instances,
        performance_definitions: &performance_definitions,
        symbol,
        cleanup_schedule: &cleanup_schedule,
        configuration,
        target,
    })
}

fn prepare_program(
    program: &Program,
    target: &TargetSpec,
) -> Result<(Program, SemanticModel), NativeEmitError> {
    let targeted_program = filter_program_for_target(program, target);
    let normalized_program = normalize_program(&targeted_program);
    let semantic = analyze(&normalized_program)
        .map_err(|error| NativeEmitError(format!("semantic analysis failed: {error:?}")))?;
    let specialized_program = super::super::generic::specialize_program(
        &normalized_program,
        &semantic.generic_instances,
    )?;
    validate_cleanup_plans(&semantic)
        .map_err(|error| NativeEmitError(format!("invalid cleanup plan: {error}")))?;
    Ok((specialized_program, semantic))
}

fn collect_declarations(program: &Program) -> (Vec<&VerbDecl>, Vec<&ExternalVerbDecl>) {
    let verbs = program.declarations.iter().filter_map(declaration_verb).collect();
    let external_verbs =
        program.declarations.iter().filter_map(declaration_external_verb).collect();
    (verbs, external_verbs)
}

fn validate_entry_verb(verbs: &[&VerbDecl], symbol: &str) -> Result<(), NativeEmitError> {
    if verbs.is_empty() {
        return Err(NativeEmitError("program has no verb declarations".to_owned()));
    }
    if !verbs.iter().any(|verb| verb.name == symbol) {
        return Err(NativeEmitError(format!("entry verb `{symbol}` was not found")));
    }
    Ok(())
}

struct VerbEmission<'items, 'program> {
    program: &'program Program,
    verbs: &'items [&'program VerbDecl],
    external_verbs: &'items [&'program ExternalVerbDecl],
    generic_instances: &'items [GenericInstance],
    performance_definitions: &'items [super::super::performance::PerformanceDefinition<'program>],
    symbol: &'items str,
    cleanup_schedule: &'items NativeCleanupSchedule,
    configuration: &'items NativeBackendConfiguration,
    target: &'items TargetSpec,
}

fn emit_verbs_object(inputs: VerbEmission<'_, '_>) -> Result<Vec<u8>, NativeEmitError> {
    let mut module = create_module(inputs.configuration, inputs.target)?;
    let (layouts, metadata) = build_layouts_and_metadata(&mut module, &inputs)?;
    let string_data = define_string_data(&mut module, inputs.verbs).map_err(NativeEmitError)?;
    let functions = function_metadata(&metadata);
    let vtable_data =
        define_vtable_data(&mut module, inputs.performance_definitions, &functions, &layouts)?;
    define_emission_functions(EmissionDefinitions {
        module: &mut module,
        verbs: inputs.verbs,
        performance_definitions: inputs.performance_definitions,
        functions: &functions,
        cleanup_schedule: inputs.cleanup_schedule,
        string_data: &string_data,
        layouts: &layouts,
        vtable_data: &vtable_data,
    })?;
    module.finish().emit().map_err(|error| NativeEmitError(error.to_string()))
}

fn build_layouts_and_metadata(
    module: &mut cranelift_object::ObjectModule,
    inputs: &VerbEmission<'_, '_>,
) -> Result<(LayoutRegistry, std::collections::HashMap<String, super::FunctionMeta>), NativeEmitError>
{
    let layouts = build_layouts(module, inputs.program, inputs.generic_instances)?;
    let metadata = declare_emission_functions(
        module,
        inputs.verbs,
        inputs.external_verbs,
        inputs.performance_definitions,
        inputs.symbol,
        &layouts,
        inputs.target,
    )?;
    Ok((layouts, metadata))
}

fn define_vtable_data(
    module: &mut cranelift_object::ObjectModule,
    performance_definitions: &[super::super::performance::PerformanceDefinition<'_>],
    functions: &std::collections::HashMap<String, super::FunctionMeta>,
    layouts: &LayoutRegistry,
) -> Result<super::super::vtable::VtableDataIds, NativeEmitError> {
    super::super::vtable::define_vtables(module, performance_definitions, functions, layouts)
}

#[allow(clippy::too_many_arguments)]
fn declare_emission_functions(
    module: &mut cranelift_object::ObjectModule,
    verbs: &[&VerbDecl],
    external_verbs: &[&ExternalVerbDecl],
    performance_definitions: &[super::super::performance::PerformanceDefinition<'_>],
    symbol: &str,
    layouts: &LayoutRegistry,
    target: &TargetSpec,
) -> Result<std::collections::HashMap<String, super::FunctionMeta>, NativeEmitError> {
    validate_native_program(verbs, external_verbs, layouts)?;
    declare_all_functions(
        module,
        verbs,
        external_verbs,
        performance_definitions,
        symbol,
        layouts,
        target,
    )
}

struct EmissionDefinitions<'items, 'program> {
    module: &'items mut cranelift_object::ObjectModule,
    verbs: &'items [&'program VerbDecl],
    performance_definitions: &'items [super::super::performance::PerformanceDefinition<'program>],
    functions: &'items std::collections::HashMap<String, super::FunctionMeta>,
    cleanup_schedule: &'items NativeCleanupSchedule,
    string_data: &'items super::super::literals::StringDataIds,
    layouts: &'items LayoutRegistry,
    vtable_data: &'items super::super::vtable::VtableDataIds,
}

fn define_emission_functions(
    mut inputs: EmissionDefinitions<'_, '_>,
) -> Result<(), NativeEmitError> {
    let frontend_config = inputs.module.isa().frontend_config();
    define_verb_bodies(&mut inputs, frontend_config)?;
    define_performance_bodies(inputs, frontend_config)
}

fn define_verb_bodies(
    inputs: &mut EmissionDefinitions<'_, '_>,
    frontend_config: cranelift_codegen::isa::TargetFrontendConfig,
) -> Result<(), NativeEmitError> {
    define_verbs(
        inputs.module,
        frontend_config,
        inputs.verbs,
        inputs.functions,
        inputs.cleanup_schedule,
        inputs.string_data,
        inputs.layouts,
        inputs.vtable_data,
    )
}

fn define_performance_bodies(
    inputs: EmissionDefinitions<'_, '_>,
    frontend_config: cranelift_codegen::isa::TargetFrontendConfig,
) -> Result<(), NativeEmitError> {
    define_performances(
        inputs.module,
        frontend_config,
        inputs.performance_definitions,
        inputs.functions,
        inputs.cleanup_schedule,
        inputs.string_data,
        inputs.layouts,
        inputs.vtable_data,
    )
}

fn build_layouts(
    module: &mut cranelift_object::ObjectModule,
    program: &Program,
    generic_instances: &[GenericInstance],
) -> Result<LayoutRegistry, NativeEmitError> {
    GenericLayoutRegistry::from_program(
        program,
        generic_instances,
        module.isa().pointer_type().bytes(),
    )?;
    LayoutRegistry::from_program_with_instances(
        program,
        module.isa().pointer_type(),
        generic_instances,
    )
}
