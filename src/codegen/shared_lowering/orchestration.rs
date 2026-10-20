use crate::ast::{Program, TopLevelDecl, TypeName};
use crate::target::{EntryContract, TargetSpec};
use std::collections::BTreeSet;

use super::declarations::{SharedDeclarations, build_backend_externals, scalar_external_symbols};
use super::layout::ProgramLayouts;
use super::performance::lower_shared_functions;
use super::runtime::{
    append_buffer_allocate_external, append_buffer_drop_external, append_format_externals,
    append_performance_externals, append_print_externals, referenced_runtime_symbols,
};
use super::runtime_externals;
use super::support::unsupported;
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
    let target = TargetSpec::host()
        .map_err(|error| BackendContractError::InvalidCleanupContract(error.to_string()))?;
    lower_program_to_backend_module_for_target(namespace, program, entry_name, bindings, &target)
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
) -> Result<BackendModule, BackendContractError> {
    let layouts = ProgramLayouts::new(program, namespace);
    let analysis_program = performance_context
        .map(|(_, context)| program_with_performance_context(program, context))
        .unwrap_or_else(|| program.clone());
    let semantic = crate::semantic::analyze(&analysis_program).map_err(|error| {
        unsupported(
            crate::lexer::SourceSpan::new(0, 0),
            format!("semantic analysis failed: {error:?}"),
        )
    })?;
    reject_floating_point_on_freestanding(&analysis_program, &semantic, target, entry_contract)?;
    reject_implicit_large_aggregate_allocation(
        &semantic,
        &ProgramLayouts::new(&analysis_program, namespace),
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
    )?;
    for function in &mut module.functions {
        function.hosted_entry_return = entry_contract == EntryContract::Hosted
            && entry_name == Some(function.symbol.as_str())
            && function.return_value.ty == BackendValueType::Unit;
    }
    module.validate()?;
    Ok(module)
}

fn program_with_performance_context(program: &Program, context: &Program) -> Program {
    let mut declarations = program.declarations.clone();
    let mut known: BTreeSet<String> =
        declarations.iter().filter_map(context_declaration_identity).collect();
    declarations.extend(
        context
            .declarations
            .iter()
            .filter(|declaration| context_declaration_identity(declaration).is_some())
            .filter(|declaration| {
                context_declaration_identity(declaration)
                    .is_some_and(|identity| known.insert(identity))
            })
            .cloned(),
    );
    Program { file_metadata: program.file_metadata.clone(), declarations }
}

fn context_declaration_identity(declaration: &TopLevelDecl) -> Option<String> {
    let (kind, name) = match declaration {
        TopLevelDecl::Constant(value) => ("constant", value.name.clone()),
        TopLevelDecl::Verb(value) => ("verb", value.name.clone()),
        TopLevelDecl::ExternalVerb(value) => ("verb", value.name.clone()),
        TopLevelDecl::Struct(value) => ("struct", value.name.clone()),
        TopLevelDecl::Pack(value) => ("pack", value.name.clone()),
        TopLevelDecl::Serialize(value) => ("serialize", value.name.clone()),
        TopLevelDecl::Enum(value) => ("enum", value.name.clone()),
        TopLevelDecl::Role(value) => ("role", value.name.clone()),
        TopLevelDecl::Perform(value) => {
            return Some(format!("perform:{}:{}", value.role_name, value.target.canonical_key()));
        }
        TopLevelDecl::OpenSibling(_) | TopLevelDecl::Import(_) => return None,
    };
    Some(format!("{kind}:{name}"))
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
    lower_program_to_backend_module_with_layouts_and_context(
        namespace, program, entry_name, bindings, layouts, None, program,
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
) -> Result<BackendModule, BackendContractError> {
    let declaration_layouts = ProgramLayouts::new(analysis_program, namespace);
    let declarations =
        SharedDeclarations::new(namespace, program, &declaration_layouts, performance_context)?;
    let semantic = crate::semantic::analyze(analysis_program).map_err(|error| {
        unsupported(
            crate::lexer::SourceSpan::new(0, 0),
            format!("semantic analysis failed: {error:?}"),
        )
    })?;
    let cleanup_provider = ProgramLayouts::new(analysis_program, namespace);
    let cleanup_plans = super::lower_semantic_cleanup_with_provider(&semantic, &cleanup_provider)?;
    let external_symbols = scalar_external_symbols(namespace, &declarations.externals, bindings)?;
    let functions = lower_shared_functions(
        namespace,
        entry_name,
        layouts,
        &declarations,
        &cleanup_plans,
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
    module.validate()?;
    Ok(module)
}
