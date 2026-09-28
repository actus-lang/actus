use crate::ast::{ExternalVerbDecl, Program, VerbDecl};
use crate::configuration::NativeBackendConfiguration;
use crate::semantic::{GenericInstance, analyze, filter_program_for_target};
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
    let targeted_program = filter_program_for_target(program, target);
    let normalized_program = normalize_program(&targeted_program);
    let program = &normalized_program;
    let semantic = analyze(program)
        .map_err(|error| NativeEmitError(format!("semantic analysis failed: {error:?}")))?;
    let codegen_program =
        super::super::generic::specialize_program(program, &semantic.generic_instances)?;
    let program = &codegen_program;
    validate_cleanup_plans(&semantic)
        .map_err(|error| NativeEmitError(format!("invalid cleanup plan: {error}")))?;
    let cleanup_schedule = NativeCleanupSchedule::from_model(&semantic);
    let performance_registry =
        PerformanceRegistry::from_program(program, &semantic.reachable_performances);
    performance_registry.validate().map_err(NativeEmitError)?;
    let performance_definitions = performance_registry.definitions(program)?;
    let verbs = program.declarations.iter().filter_map(declaration_verb).collect::<Vec<_>>();
    let external_verbs =
        program.declarations.iter().filter_map(declaration_external_verb).collect::<Vec<_>>();
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
    performance_definitions: &[super::super::performance::PerformanceDefinition<'_>],
    symbol: &str,
    cleanup_schedule: &NativeCleanupSchedule,
    configuration: &NativeBackendConfiguration,
    target: &TargetSpec,
) -> Result<Vec<u8>, NativeEmitError> {
    let mut module = create_module(configuration, target)?;
    let layouts = build_layouts(&mut module, program, generic_instances)?;
    validate_native_program(verbs, external_verbs, &layouts)?;
    let frontend_config = module.isa().frontend_config();
    let metadata = declare_all_functions(
        &mut module,
        verbs,
        external_verbs,
        performance_definitions,
        symbol,
        &layouts,
        target,
    )?;
    let string_data = define_string_data(&mut module, verbs).map_err(NativeEmitError)?;
    let functions = function_metadata(&metadata);
    let vtable_data = super::super::vtable::define_vtables(
        &mut module,
        performance_definitions,
        &functions,
        &layouts,
    )?;
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
