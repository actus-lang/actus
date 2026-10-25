use std::collections::BTreeMap;

use crate::ast::{ExternalVerbDecl, TypeName};
use crate::ast::{PerformDecl, VerbDecl};

use super::super::{
    BackendBlockId, BackendCleanupPlan, BackendContractError, BackendFunction, BackendInstruction,
    BackendLayoutProvider, BackendSignature, BackendSymbolBindings, BackendValueType,
    SymbolIdentity,
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
    cleanup_plans: &'a [BackendCleanupPlan],
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
            ((perform.target.canonical_key(), method.name.clone()), signature.symbol.clone())
        })
        .collect();
    let return_types = performances
        .iter()
        .zip(signatures)
        .map(|((perform, method), signature)| {
            (
                (perform.target.canonical_key(), method.name.clone()),
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
                (perform.target.canonical_key(), method.name.clone()),
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
    cleanup_plans: &[BackendCleanupPlan],
    external_symbols: &BTreeMap<String, SymbolIdentity>,
    bindings: Option<&BackendSymbolBindings>,
) -> Result<Vec<BackendFunction>, BackendContractError> {
    let context = PerformanceLoweringContext {
        namespace,
        layouts,
        declarations,
        cleanup_plans,
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
            cleanup_plans,
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
        context.cleanup_plans,
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
        .cleanup_plans
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
            let name = match action {
                super::super::BackendCleanupAction::DropBinding { name, .. }
                | super::super::BackendCleanupAction::DropBindingFields { name, .. } => name,
                _ => continue,
            };
            if !provider.parameter_is_owned(name) {
                continue;
            }
            if cleanup.iter().any(|(existing, _, _, _)| existing == name) {
                continue;
            }
            let Some(value) = provider.parameter_value(name) else { continue };
            if let Some(descriptors) = plan.resolved_bridges.get(index) {
                for descriptor in descriptors {
                    if !super::expression::descriptor_applies(provider, value, descriptor) {
                        continue;
                    }
                    cleanup.push((
                        name.clone(),
                        action.clone(),
                        descriptor.operand(value),
                        plan.span,
                    ));
                }
            }
        }
    }
    cleanup
}
