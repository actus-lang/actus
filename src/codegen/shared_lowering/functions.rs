use std::collections::BTreeMap;

use crate::ast::{TypeName, VerbDecl};

use super::errors::unsupported;
use super::expression::SharedExpressionProvider;
use super::performance;
use super::{
    BackendBlockId, BackendContractError, BackendLayoutProvider, BackendPassingMode,
    BackendSymbolBindings, BackendValueType, SymbolIdentity, SymbolKind,
    lower_verb_with_expressions, lower_verb_with_expressions_as,
};

pub(crate) fn lower_shared_verb(
    namespace: &str,
    verb: &VerbDecl,
    entry_name: Option<&str>,
    layouts: &impl BackendLayoutProvider,
    cleanup_plans: &[super::BackendCleanupPlan],
    return_types: &BTreeMap<String, Option<BackendValueType>>,
    return_nominal_types: &BTreeMap<String, Option<TypeName>>,
    return_payload_types: &BTreeMap<String, [Option<BackendValueType>; 2]>,
    performance_symbols: &BTreeMap<(String, String), SymbolIdentity>,
    performance_return_types: &BTreeMap<(String, String), Option<BackendValueType>>,
    performance_return_nominal_types: &BTreeMap<(String, String), Option<TypeName>>,
    parameter_modes: &BTreeMap<String, Vec<BackendPassingMode>>,
    parameter_types: &BTreeMap<String, Vec<BackendValueType>>,
    external_symbols: &BTreeMap<String, SymbolIdentity>,
    bindings: Option<&BackendSymbolBindings>,
) -> Result<super::BackendFunction, BackendContractError> {
    let function_symbol = symbol_for_verb(namespace, verb, entry_name)?;
    let mut provider = SharedExpressionProvider::new(
        namespace,
        verb,
        function_symbol.clone(),
        layouts,
        return_types,
        return_nominal_types,
        return_payload_types,
        parameter_modes,
        parameter_types,
        performance_symbols,
        performance_return_types,
        performance_return_nominal_types,
        external_symbols,
        bindings,
        cleanup_plans,
    );
    let mut function = match entry_name.filter(|name| *name == verb.name) {
        Some(_) => lower_verb_with_expressions_as(
            function_symbol,
            verb,
            layouts,
            &mut provider,
            BackendBlockId(0),
        ),
        None => {
            lower_verb_with_expressions(namespace, verb, layouts, &mut provider, BackendBlockId(0))
        }
    }?;
    if let Some(entry) = function.blocks.first_mut() {
        let mut prologue = provider.scalar_loan_prologue();
        prologue.append(&mut entry.instructions);
        entry.instructions = prologue;
    }
    assign_cleanup_and_performance(&mut function, verb, cleanup_plans, &provider);
    function.validate()?;
    Ok(function)
}

fn symbol_for_verb(
    namespace: &str,
    verb: &VerbDecl,
    entry_name: Option<&str>,
) -> Result<SymbolIdentity, BackendContractError> {
    match entry_name.filter(|name| *name == verb.name) {
        Some(_) => SymbolIdentity::entry(&verb.name),
        None => SymbolIdentity::new(namespace, SymbolKind::Verb, &verb.name),
    }
    .map_err(|error| unsupported(verb.span, error.to_string()))
}

fn assign_cleanup_and_performance(
    function: &mut super::BackendFunction,
    verb: &VerbDecl,
    cleanup_plans: &[super::BackendCleanupPlan],
    provider: &SharedExpressionProvider<'_>,
) {
    function.cleanup = cleanup_plans
        .iter()
        .filter(|plan| plan.span.start >= verb.span.start && plan.span.end <= verb.span.end)
        .cloned()
        .collect();
    performance::append_parameter_cleanup(function, provider);
}
