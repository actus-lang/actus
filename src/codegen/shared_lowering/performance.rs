use std::collections::BTreeMap;

use crate::ast::{ExternalVerbDecl, TypeName};
use crate::ast::{PerformDecl, VerbDecl};

use super::super::{
    BackendBlockId, BackendContractError, BackendFunction, BackendLayoutProvider, BackendSignature,
    BackendSymbolBindings, BackendValueType, SymbolIdentity,
};
use super::expression::SharedExpressionProvider;
use super::{SharedDeclarations, lower_shared_verb};

type PerformanceDispatchMaps = (
    BTreeMap<(String, String), SymbolIdentity>,
    BTreeMap<(String, String), Option<BackendValueType>>,
    BTreeMap<(String, String), Option<TypeName>>,
);

struct PerformanceLoweringContext<'a, 'program, L: BackendLayoutProvider + ?Sized> {
    namespace: &'a str,
    layouts: &'a L,
    declarations: &'a SharedDeclarations<'program>,
    cleanup: super::CleanupInputs<'a>,
    external_symbols: &'a BTreeMap<String, SymbolIdentity>,
    bindings: Option<&'a BackendSymbolBindings>,
}

pub(super) fn performance_dispatch_maps(
    performances: &[(&PerformDecl, &VerbDecl)],
    signatures: &[BackendSignature],
) -> PerformanceDispatchMaps {
    let symbols = performances
        .iter()
        .zip(signatures)
        .map(|((perform, method), signature)| {
            ((perform.target.canonical_lookup_key(), method.name.clone()), signature.symbol.clone())
        })
        .collect();
    let return_types = performances
        .iter()
        .zip(signatures)
        .map(|((perform, method), signature)| {
            (
                (perform.target.canonical_lookup_key(), method.name.clone()),
                (signature.return_value.ty != BackendValueType::Unit)
                    .then_some(signature.return_value.ty),
            )
        })
        .collect();
    let return_nominal_types = performances
        .iter()
        .zip(signatures)
        .map(|((perform, method), _)| {
            (
                (perform.target.canonical_lookup_key(), method.name.clone()),
                method.return_type.as_ref().map(|return_type| return_type.ty.clone()),
            )
        })
        .collect();
    (symbols, return_types, return_nominal_types)
}

pub(super) fn shared_return_nominal_types(
    verbs: &[&VerbDecl],
    externals: &[&ExternalVerbDecl],
    performances: &[(&PerformDecl, &VerbDecl)],
) -> BTreeMap<String, Option<TypeName>> {
    verbs
        .iter()
        .map(|verb| {
            (verb.name.clone(), verb.return_type.as_ref().map(|return_type| return_type.ty.clone()))
        })
        .chain(externals.iter().map(|external| {
            (
                external.name.clone(),
                external.return_type.as_ref().map(|return_type| return_type.ty.clone()),
            )
        }))
        .chain(performances.iter().map(|(_, method)| {
            (
                method.name.clone(),
                method.return_type.as_ref().map(|return_type| return_type.ty.clone()),
            )
        }))
        .collect()
}

pub(super) fn lower_shared_functions(
    namespace: &str,
    entry_name: Option<&str>,
    layouts: &impl BackendLayoutProvider,
    declarations: &SharedDeclarations<'_>,
    cleanup: super::CleanupInputs<'_>,
    external_symbols: &BTreeMap<String, SymbolIdentity>,
    bindings: Option<&BackendSymbolBindings>,
) -> Result<Vec<BackendFunction>, BackendContractError> {
    let context = PerformanceLoweringContext {
        namespace,
        layouts,
        declarations,
        cleanup,
        external_symbols,
        bindings,
    };
    let mut functions = Vec::with_capacity(
        declarations.verbs.len()
            + declarations
                .performances
                .iter()
                .filter(|(perform, _)| !perform.module_import)
                .count(),
    );
    for verb in &declarations.verbs {
        functions.push(lower_shared_verb(
            namespace,
            verb,
            entry_name,
            layouts,
            cleanup,
            &declarations.return_types,
            &declarations.return_nominal_types,
            &declarations.return_payload_types,
            &declarations.performance_symbols,
            &declarations.performance_return_types,
            &declarations.performance_return_nominal_types,
            &declarations.parameter_modes,
            &declarations.parameter_types,
            external_symbols,
            bindings,
        )?);
    }
    for ((perform, method), signature) in declarations
        .performances
        .iter()
        .zip(&declarations.performance_signatures)
        .filter(|((perform, _), _)| !perform.module_import)
    {
        functions.push(lower_shared_performance(
            &context,
            perform,
            method,
            signature.symbol.clone(),
        )?);
    }
    Ok(functions)
}

fn lower_shared_performance<L: BackendLayoutProvider>(
    context: &PerformanceLoweringContext<'_, '_, L>,
    _perform: &PerformDecl,
    method: &VerbDecl,
    symbol: SymbolIdentity,
) -> Result<BackendFunction, BackendContractError> {
    let mut provider = SharedExpressionProvider::new(
        context.namespace,
        method,
        symbol.clone(),
        context.layouts,
        &context.declarations.return_types,
        &context.declarations.return_nominal_types,
        &context.declarations.return_payload_types,
        &context.declarations.parameter_modes,
        &context.declarations.parameter_types,
        &context.declarations.performance_symbols,
        &context.declarations.performance_return_types,
        &context.declarations.performance_return_nominal_types,
        context.external_symbols,
        context.bindings,
        context.cleanup.1.verb(&method.name, method.span),
    );
    let mut function = super::lower_verb_with_expressions_as(
        symbol,
        method,
        context.layouts,
        &mut provider,
        BackendBlockId(0),
    )?;
    if let Some(entry) = function.blocks.first_mut() {
        let mut prologue = provider.scalar_loan_prologue();
        prologue.append(&mut entry.instructions);
        entry.instructions = prologue;
    }
    function.cleanup = context
        .cleanup
        .0
        .iter()
        .filter(|plan| plan.span.start >= method.span.start && plan.span.end <= method.span.end)
        .cloned()
        .collect();
    function.validate()?;
    Ok(function)
}
