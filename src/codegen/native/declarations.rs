use std::collections::HashMap;

use cranelift_object::ObjectModule;

use crate::ast::{ExternalVerbDecl, TopLevelDecl, VerbDecl};
use crate::target::TargetSpec;

use super::super::abi::{validate_external_native_signature, validate_native_signature};
use super::super::declarations::declare_functions;
use super::super::function_definition::define_function;
use super::super::layout::LayoutRegistry;
use super::super::literals::StringDataIds;
use super::super::model::NativeCleanupSchedule;
use super::super::native_runtime::declare_runtime_functions;
use super::{FunctionMeta, NativeEmitError};

pub(super) fn declaration_verb(declaration: &TopLevelDecl) -> Option<&VerbDecl> {
    match declaration {
        TopLevelDecl::Verb(verb) if verb.generic_parameters.is_empty() => Some(verb),
        _ => None,
    }
}

pub(super) fn declaration_external_verb(declaration: &TopLevelDecl) -> Option<&ExternalVerbDecl> {
    match declaration {
        TopLevelDecl::ExternalVerb(verb) => Some(verb),
        _ => None,
    }
}

pub(super) struct DeclarationContext<'a> {
    pub(super) entry_symbol: &'a str,
    pub(super) namespace_prefix: &'a str,
    pub(super) layouts: &'a LayoutRegistry,
    pub(super) target: &'a TargetSpec,
}

pub(super) fn declare_all_functions(
    module: &mut ObjectModule,
    verbs: &[&VerbDecl],
    external_verbs: &[&ExternalVerbDecl],
    performance_definitions: &[super::super::performance::PerformanceDefinition<'_>],
    context: DeclarationContext<'_>,
) -> Result<HashMap<String, FunctionMeta>, NativeEmitError> {
    let mut metadata = declare_functions(
        module,
        verbs,
        external_verbs,
        context.entry_symbol,
        context.namespace_prefix,
        context.layouts,
    )?;
    metadata.extend(super::super::performance::declare_performance_functions(
        module,
        performance_definitions,
        context.layouts,
    )?);
    let has_print_definition = verbs.iter().any(|verb| verb.name == "print")
        || external_verbs.iter().any(|verb| verb.name == "print");
    if target_requires_host_runtime(context.target) {
        metadata.extend(declare_runtime_functions(module, has_print_definition)?);
    }
    Ok(metadata)
}

pub(super) fn target_requires_host_runtime(target: &TargetSpec) -> bool {
    matches!(target.entry_contract(), crate::target::EntryContract::Hosted)
}

pub(super) fn validate_native_program(
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

pub(super) fn function_metadata(
    metadata: &HashMap<String, FunctionMeta>,
) -> HashMap<String, FunctionMeta> {
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
                    ins_params: meta.ins_params.clone(),
                },
            )
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn define_verbs(
    module: &mut ObjectModule,
    frontend_config: cranelift_codegen::isa::TargetFrontendConfig,
    verbs: &[&VerbDecl],
    functions: &HashMap<String, FunctionMeta>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataIds,
    layouts: &LayoutRegistry,
    vtable_data: &super::super::vtable::VtableDataIds,
    namespace_prefix: &str,
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
            namespace_prefix,
        )?;
    }
    Ok(())
}
