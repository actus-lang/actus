use std::collections::BTreeMap;

use crate::ast::{TypeName, VerbDecl};

use super::expression::SharedExpressionProvider;
use super::performance;
use super::support::unsupported;
use super::{
    BackendBlockId, BackendContractError, BackendLayoutProvider, BackendPassingMode,
    BackendSymbolBindings, BackendValueType, SymbolIdentity, lower_verb_with_expressions,
    lower_verb_with_expressions_as,
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
    let mut provider = SharedExpressionProvider::new(
        namespace,
        verb,
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
            SymbolIdentity::entry(&verb.name)
                .map_err(|error| unsupported(verb.span, error.to_string()))?,
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
    let writebacks = provider.scalar_loan_writebacks();
    if !writebacks.is_empty() {
        for block in &mut function.blocks {
            let Some(index) = block.instructions.iter().position(|instruction| {
                matches!(
                    instruction,
                    super::BackendInstruction::Return { .. }
                        | super::BackendInstruction::Fail { .. }
                )
            }) else {
                continue;
            };
            block.instructions.splice(index..index, writebacks.clone());
        }
    }
    function.cleanup = cleanup_plans
        .iter()
        .filter(|plan| plan.span.start >= verb.span.start && plan.span.end <= verb.span.end)
        .cloned()
        .collect();
    performance::append_parameter_cleanup(&mut function, &provider);
    function.validate()?;
    Ok(function)
}
