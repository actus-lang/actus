// Performance lowering receives the complete neutral backend context.
#![allow(clippy::too_many_arguments)]

use std::collections::BTreeMap;

use crate::ast::{PerformDecl, VerbDecl};

use super::super::{
    BackendBlockId, BackendCleanupPlan, BackendContractError, BackendFunction, BackendInstruction,
    BackendLayoutProvider, BackendSymbolBindings, BackendValueType, SymbolIdentity,
};
use super::expression::SharedExpressionProvider;
use super::{SharedDeclarations, lower_shared_verb};

pub(super) fn lower_shared_functions(
    namespace: &str,
    entry_name: Option<&str>,
    layouts: &impl BackendLayoutProvider,
    declarations: &SharedDeclarations<'_>,
    cleanup_plans: &[BackendCleanupPlan],
    external_symbols: &BTreeMap<String, SymbolIdentity>,
    bindings: Option<&BackendSymbolBindings>,
) -> Result<Vec<BackendFunction>, BackendContractError> {
    let mut functions =
        Vec::with_capacity(declarations.verbs.len() + declarations.performances.len());
    for verb in &declarations.verbs {
        functions.push(lower_shared_verb(
            namespace,
            verb,
            entry_name,
            layouts,
            cleanup_plans,
            &declarations.return_types,
            &declarations.return_payload_types,
            external_symbols,
            bindings,
        )?);
    }
    for ((perform, method), signature) in
        declarations.performances.iter().zip(&declarations.performance_signatures)
    {
        functions.push(lower_shared_performance(
            namespace,
            perform,
            method,
            signature.symbol.clone(),
            layouts,
            cleanup_plans,
            &declarations.return_types,
            &declarations.return_payload_types,
            external_symbols,
            bindings,
        )?);
    }
    Ok(functions)
}

fn lower_shared_performance(
    namespace: &str,
    _perform: &PerformDecl,
    method: &VerbDecl,
    symbol: SymbolIdentity,
    layouts: &impl BackendLayoutProvider,
    cleanup_plans: &[BackendCleanupPlan],
    return_types: &BTreeMap<String, Option<BackendValueType>>,
    return_payload_types: &BTreeMap<String, [Option<BackendValueType>; 2]>,
    external_symbols: &BTreeMap<String, SymbolIdentity>,
    bindings: Option<&BackendSymbolBindings>,
) -> Result<BackendFunction, BackendContractError> {
    let mut provider = SharedExpressionProvider::new(
        namespace,
        method,
        layouts,
        return_types,
        return_payload_types,
        external_symbols,
        bindings,
        cleanup_plans,
    );
    let mut function = super::lower_verb_with_expressions_as(
        symbol,
        method,
        layouts,
        &mut provider,
        BackendBlockId(0),
    )?;
    function.cleanup = cleanup_plans
        .iter()
        .filter(|plan| plan.span.start >= method.span.start && plan.span.end <= method.span.end)
        .cloned()
        .collect();
    append_parameter_cleanup(&mut function, &provider);
    function.validate()?;
    Ok(function)
}

type ParameterCleanup = (
    String,
    super::super::BackendCleanupAction,
    super::super::BackendCleanupOperand,
    crate::lexer::SourceSpan,
);

pub(super) fn append_parameter_cleanup(
    function: &mut BackendFunction,
    provider: &SharedExpressionProvider<'_>,
) {
    let cleanup = parameter_cleanup(function, provider);
    if cleanup.is_empty() {
        return;
    }
    for block in &mut function.blocks {
        let instructions = std::mem::take(&mut block.instructions);
        let mut lowered = Vec::with_capacity(instructions.len() + cleanup.len());
        for instruction in instructions {
            let returned_value = match &instruction {
                BackendInstruction::Return { value, .. } => *value,
                BackendInstruction::Fail { .. } => None,
                _ => None,
            };
            if matches!(
                instruction,
                BackendInstruction::Return { .. } | BackendInstruction::Fail { .. }
            ) {
                for (name, action, operand, span) in &cleanup {
                    if returned_value == provider.parameter_value(name) {
                        continue;
                    }
                    lowered.push(BackendInstruction::Cleanup {
                        action: action.clone(),
                        operand: Some(operand.clone()),
                        span: *span,
                    });
                }
            }
            lowered.push(instruction);
        }
        block.instructions = lowered;
    }
}

fn parameter_cleanup(
    function: &BackendFunction,
    provider: &SharedExpressionProvider<'_>,
) -> Vec<ParameterCleanup> {
    let mut cleanup = Vec::new();
    for plan in function
        .cleanup
        .iter()
        .filter(|plan| !matches!(plan.origin, super::super::BackendCleanupOrigin::Explicit { .. }))
    {
        for (index, action) in plan.actions.iter().enumerate() {
            let super::super::BackendCleanupAction::DropBinding { name, .. } = action else {
                continue;
            };
            if !provider.parameter_is_owned(name) {
                continue;
            }
            let Some(value) = provider.parameter_value(name) else { continue };
            let Some(descriptor) = plan.resolved_bridges.get(index).and_then(|items| items.first())
            else {
                continue;
            };
            if cleanup.iter().any(|(existing, _, _, _)| existing == name) {
                continue;
            }
            cleanup.push((name.clone(), action.clone(), descriptor.operand(value), plan.span));
        }
    }
    cleanup
}
