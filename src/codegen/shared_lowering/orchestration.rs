use crate::ast::{Program, TopLevelDecl, TypeName};
use crate::target::{EntryContract, TargetSpec};

use super::declarations::{SharedDeclarations, build_backend_externals, scalar_external_symbols};
use super::errors::unsupported;
use super::layout::ProgramLayouts;
use super::performance::lower_shared_functions;
use super::runtime::{
    append_buffer_allocate_external, append_buffer_drop_external, append_format_externals,
    append_performance_externals, append_print_externals, referenced_runtime_symbols,
};
use super::runtime_externals;
use super::{
    BackendContractError, BackendLayoutProvider, BackendModule, BackendSymbolBindings,
    BackendValueType,
};

#[doc(hidden)]
pub fn lower_scalar_program(
    namespace: &str,
    program: &Program,
    entry_name: Option<&str>,
) -> Result<BackendModule, BackendContractError> {
    lower_program_to_backend_module(namespace, program, entry_name, None)
}

pub(crate) fn lower_program_to_backend_module(
    namespace: &str,
    program: &Program,
    entry_name: Option<&str>,
    bindings: Option<&BackendSymbolBindings>,
) -> Result<BackendModule, BackendContractError> {
    let mut standalone = program.clone();
    crate::modules::DeclarationRegistry::resolve_standalone(&mut standalone);
    let target = TargetSpec::host()
        .map_err(|error| BackendContractError::InvalidCleanupContract(error.to_string()))?;
    lower_program_to_backend_module_for_target(
        namespace,
        &standalone,
        entry_name,
        bindings,
        &target,
    )
}

pub(crate) fn lower_program_to_backend_module_for_target(
    namespace: &str,
    program: &Program,
    entry_name: Option<&str>,
    bindings: Option<&BackendSymbolBindings>,
    target: &TargetSpec,
) -> Result<BackendModule, BackendContractError> {
    lower_program_to_backend_module_for_target_with_context(
        namespace,
        program,
        entry_name,
        bindings,
        target,
        target.entry_contract(),
        None,
        crate::modules::aggregation::registry::STANDALONE_PACKAGE_IDENTITY,
        crate::modules::aggregation::registry::standalone_module_path(),
    )
}

pub(crate) fn lower_program_to_backend_module_for_target_with_context(
    namespace: &str,
    program: &Program,
    entry_name: Option<&str>,
    bindings: Option<&BackendSymbolBindings>,
    target: &TargetSpec,
    entry_contract: EntryContract,
    performance_context: Option<(&str, &Program)>,
    package: &str,
    module_path: &str,
) -> Result<BackendModule, BackendContractError> {
    let layouts = ProgramLayouts::new(program, namespace, package, module_path);
    let analysis_program = performance_context
        .map(|(_, context)| crate::modules::program_with_performance_context(program, context))
        .unwrap_or_else(|| program.clone());
    let semantic = crate::semantic::analyze_resolved(&analysis_program).map_err(|error| {
        let diagnostic = crate::diagnostics::semantic_diagnostic(&error);
        unsupported(
            crate::lexer::SourceSpan::new(0, 0),
            format!(
                "internal compiler error: semantic validation invariant violated: {} {}",
                diagnostic.code(),
                diagnostic.message()
            ),
        )
    })?;
    reject_floating_point_on_freestanding(&analysis_program, &semantic, target, entry_contract)?;
    reject_implicit_large_aggregate_allocation(
        &semantic,
        &ProgramLayouts::new(&analysis_program, namespace, package, module_path),
        target,
        entry_contract,
    )?;
    let mut module = lower_program_to_backend_module_with_layouts_and_context(
        namespace,
        program,
        entry_name,
        bindings,
        &layouts,
        performance_context,
        &analysis_program,
        package,
        module_path,
    )?;
    for function in &mut module.functions {
        function.hosted_entry_return = entry_contract == EntryContract::Hosted
            && entry_name == Some(function.symbol.as_str())
            && function.return_value.ty == BackendValueType::Unit;
    }
    module.validate()?;
    Ok(module)
}

fn reject_floating_point_on_freestanding(
    program: &Program,
    semantic: &crate::semantic::SemanticModel,
    target: &TargetSpec,
    entry_contract: EntryContract,
) -> Result<(), BackendContractError> {
    if entry_contract == EntryContract::Hosted {
        return Ok(());
    }
    let Some(span) = find_floating_point_span(program, semantic) else { return Ok(()) };
    Err(BackendContractError::Unsupported(super::BackendUnsupportedOperation::new(
        "floating-point lowering",
        span,
        super::BackendTargetContext {
            backend: "shared".to_owned(),
            target: target.triple().to_string(),
        },
        "floating-point values are forbidden by the freestanding target profile; use an integer or fixed-point representation",
    )))
}

fn find_floating_point_span(
    program: &Program,
    semantic: &crate::semantic::SemanticModel,
) -> Option<crate::lexer::SourceSpan> {
    let is_float = |type_name: &TypeName| {
        matches!(
            crate::ast::primitive_type(&type_name.name),
            Some(crate::ast::PrimitiveType::Float { .. })
        )
    };
    if let Some(fact) = semantic.literal_facts.iter().find(|fact| fact.kind == "float") {
        return Some(fact.span);
    }
    program.declarations.iter().find_map(|declaration| {
        let type_name = match declaration {
            TopLevelDecl::Verb(verb) => verb
                .params
                .iter()
                .map(|parameter| &parameter.ty)
                .chain(verb.return_type.iter().map(|return_type| &return_type.ty))
                .find(|type_name| is_float(type_name)),
            TopLevelDecl::ExternalVerb(verb) => verb
                .params
                .iter()
                .map(|parameter| &parameter.ty)
                .chain(verb.return_type.iter().map(|return_type| &return_type.ty))
                .find(|type_name| is_float(type_name)),
            TopLevelDecl::Struct(definition) => definition
                .fields
                .iter()
                .map(|field| &field.ty)
                .find(|type_name| is_float(type_name)),
            TopLevelDecl::Enum(definition) => {
                definition.variants.iter().find_map(|variant| match &variant.payload {
                    crate::ast::EnumPayload::Tuple(types) => types.iter().find(|ty| is_float(ty)),
                    crate::ast::EnumPayload::Struct(fields) => {
                        fields.iter().map(|field| &field.ty).find(|ty| is_float(ty))
                    }
                    crate::ast::EnumPayload::Unit => None,
                })
            }
            _ => None,
        };
        type_name.map(|type_name| type_name.span)
    })
}

fn reject_implicit_large_aggregate_allocation(
    semantic: &crate::semantic::SemanticModel,
    layouts: &ProgramLayouts<'_>,
    target: &TargetSpec,
    entry_contract: EntryContract,
) -> Result<(), BackendContractError> {
    if entry_contract == EntryContract::Hosted {
        return Ok(());
    }
    for (binding_index, type_name) in &semantic.binding_type_names {
        let Some(layout) = layouts.layout_for(type_name) else { continue };
        if layout.size < super::LARGE_AGGREGATE_THRESHOLD {
            continue;
        }
        let span = semantic
            .bindings
            .get(*binding_index)
            .map(|binding| binding.span)
            .unwrap_or(type_name.span);
        return Err(BackendContractError::Unsupported(super::BackendUnsupportedOperation::new(
            "large aggregate implicit allocation",
            span,
            super::BackendTargetContext {
                backend: "shared".to_owned(),
                target: target.triple().to_string(),
            },
            format!(
                "aggregate layout size {} exceeds target stack limit, and implicit dynamic allocation is forbidden on freestanding targets",
                layout.size
            ),
        )));
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn lower_program_to_backend_module_with_layouts(
    namespace: &str,
    program: &Program,
    entry_name: Option<&str>,
    bindings: Option<&BackendSymbolBindings>,
    layouts: &impl BackendLayoutProvider,
) -> Result<BackendModule, BackendContractError> {
    let mut standalone = program.clone();
    crate::modules::DeclarationRegistry::resolve_standalone(&mut standalone);
    lower_program_to_backend_module_with_layouts_and_context(
        namespace,
        &standalone,
        entry_name,
        bindings,
        layouts,
        None,
        &standalone,
        crate::modules::aggregation::registry::STANDALONE_PACKAGE_IDENTITY,
        crate::modules::aggregation::registry::standalone_module_path(),
    )
}

pub(crate) fn lower_program_to_backend_module_with_layouts_and_context(
    namespace: &str,
    program: &Program,
    entry_name: Option<&str>,
    bindings: Option<&BackendSymbolBindings>,
    layouts: &impl BackendLayoutProvider,
    performance_context: Option<(&str, &Program)>,
    analysis_program: &Program,
    package: &str,
    module_path: &str,
) -> Result<BackendModule, BackendContractError> {
    let declaration_layouts =
        ProgramLayouts::new(analysis_program, namespace, package, module_path);
    let declarations =
        SharedDeclarations::new(namespace, program, &declaration_layouts, performance_context)?;
    let semantic = analyze_after_validation(analysis_program)?;
    let cleanup_provider = ProgramLayouts::new(analysis_program, namespace, package, module_path);
    let cleanup_plans = super::lower_semantic_cleanup_with_provider(&semantic, &cleanup_provider)?;
    let schedule = super::exits::CleanupSchedule::build(&semantic, &cleanup_provider)?;
    let external_symbols = scalar_external_symbols(namespace, &declarations.externals, bindings)?;
    let functions = lower_shared_functions(
        namespace,
        entry_name,
        layouts,
        &declarations,
        (&cleanup_plans, &schedule),
        &external_symbols,
        bindings,
    )?;
    let mut module = BackendModule {
        externals: build_backend_externals(
            &declarations.externals,
            declarations.external_signatures,
            &external_symbols,
        )?,
        functions,
        runtime_checks: crate::codegen::RuntimeCheckReporting::Trap,
    };
    append_performance_externals(&mut module.externals, &declarations.performance_externals)?;
    let mut runtime_symbols = referenced_runtime_symbols(&module.functions);
    if module
        .externals
        .iter()
        .any(|external| external.symbol.as_str() == crate::runtime::REGION_OPEN_SYMBOL)
    {
        runtime_symbols.insert(crate::runtime::BUFFER_DROP_SYMBOL.to_owned());
    }
    append_buffer_drop_external(&mut module.externals, &runtime_symbols)?;
    append_buffer_allocate_external(&mut module.externals, &runtime_symbols)?;
    runtime_externals::append_buffer_runtime_externals(&mut module.externals, &runtime_symbols)?;
    append_print_externals(&mut module.externals, &runtime_symbols)?;
    append_format_externals(&mut module.externals, &runtime_symbols)?;
    remove_defined_function_externals(&mut module);
    module.validate()?;
    Ok(module)
}

fn analyze_after_validation(
    program: &Program,
) -> Result<crate::semantic::SemanticModel, BackendContractError> {
    crate::semantic::analyze_resolved(program).map_err(|error| {
        let diagnostic = crate::diagnostics::semantic_diagnostic(&error);
        unsupported(
            crate::lexer::SourceSpan::new(0, 0),
            format!(
                "internal compiler error: semantic validation invariant violated: {} {}",
                diagnostic.code(),
                diagnostic.message()
            ),
        )
    })
}

fn remove_defined_function_externals(module: &mut BackendModule) {
    module.externals.retain(|external| {
        !module
            .functions
            .iter()
            .any(|function| function.symbol.as_str() == external.symbol.as_str())
    });
}
