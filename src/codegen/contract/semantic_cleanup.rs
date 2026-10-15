use super::super::model::LoweredCleanupInstruction;
use super::{
    BackendCleanupOrigin, BackendCleanupPlan, BackendCleanupProvider, BackendContractError,
};

pub fn lower_semantic_cleanup_with_provider(
    model: &crate::semantic::SemanticModel,
    provider: &impl BackendCleanupProvider,
) -> Result<Vec<BackendCleanupPlan>, BackendContractError> {
    super::super::model::validate_cleanup_plans(model)
        .map_err(BackendContractError::InvalidCleanupContract)?;
    let plans = super::super::model::lower_cleanup_plans(model)
        .into_iter()
        .map(|plan| (plan, BackendCleanupOrigin::Scope))
        .chain(model.explicit_drops.iter().filter_map(|drop| {
            super::super::model::lower_explicit_drop(drop.binding_index, model).map(|action| {
                (
                    super::super::model::LoweredCleanupPlan {
                        depth: 0,
                        span: drop.span,
                        instructions: vec![action],
                    },
                    BackendCleanupOrigin::Explicit { span: drop.span },
                )
            })
        }))
        .chain(super::super::model::lower_return_unwind_plans(model).into_iter().flat_map(
            |unwind| {
                unwind
                    .scopes
                    .into_iter()
                    .map(move |plan| (plan, BackendCleanupOrigin::Return { span: unwind.span }))
            },
        ))
        .chain(super::super::model::lower_loop_unwind_plans(model).into_iter().flat_map(
            |unwind| {
                let kind = unwind.kind.clone();
                unwind.scopes.into_iter().map(move |plan| {
                    (plan, BackendCleanupOrigin::Loop { span: unwind.span, kind: kind.clone() })
                })
            },
        ));
    plans
        .map(|(plan, origin)| lower_plan(plan, origin, model, provider))
        .collect::<Result<Vec<_>, _>>()
}

fn lower_plan(
    plan: super::super::model::LoweredCleanupPlan,
    origin: BackendCleanupOrigin,
    model: &crate::semantic::SemanticModel,
    provider: &impl BackendCleanupProvider,
) -> Result<BackendCleanupPlan, BackendContractError> {
    let resolved_bridges = plan
        .instructions
        .iter()
        .map(|action| resolve_action(action, model, provider))
        .collect::<Result<Vec<_>, _>>()?;
    let bridges = plan
        .instructions
        .iter()
        .map(|action| super::super::model::cleanup_bridge_descriptor(action, model))
        .collect();
    Ok(BackendCleanupPlan {
        depth: plan.depth,
        span: plan.span,
        origin,
        actions: plan.instructions,
        bridges,
        resolved_bridges,
    })
}

fn resolve_action(
    action: &LoweredCleanupInstruction,
    model: &crate::semantic::SemanticModel,
    provider: &impl BackendCleanupProvider,
) -> Result<Vec<super::BackendCleanupResolvedDescriptor>, BackendContractError> {
    let descriptors = match action {
        LoweredCleanupInstruction::DropBinding { binding_index, .. } => {
            resolve_binding(*binding_index, model, provider)?
        }
        LoweredCleanupInstruction::DropBindingFields { binding_index, moved_fields, .. } => {
            resolve_fields(*binding_index, moved_fields, model, provider)?
        }
        LoweredCleanupInstruction::EndBorrow { .. }
        | LoweredCleanupInstruction::DropPayloadField { .. }
        | LoweredCleanupInstruction::ResetArena { .. } => Vec::new(),
    };
    for descriptor in &descriptors {
        descriptor.validate()?;
    }
    Ok(descriptors)
}

fn resolve_binding(
    binding_index: usize,
    model: &crate::semantic::SemanticModel,
    provider: &impl BackendCleanupProvider,
) -> Result<Vec<super::BackendCleanupResolvedDescriptor>, BackendContractError> {
    let binding = model.bindings.get(binding_index).ok_or_else(|| {
        BackendContractError::InvalidCleanupContract(
            "cleanup references invalid binding".to_owned(),
        )
    })?;
    Ok(provider
        .resolve_binding(model.binding_type_names.get(&binding_index), binding.ty)?
        .into_iter()
        .collect())
}

fn resolve_fields(
    binding_index: usize,
    moved_fields: &[String],
    model: &crate::semantic::SemanticModel,
    provider: &impl BackendCleanupProvider,
) -> Result<Vec<super::BackendCleanupResolvedDescriptor>, BackendContractError> {
    let root_type = model.binding_type_names.get(&binding_index).ok_or_else(|| {
        BackendContractError::InvalidCleanupContract(
            "moved-field cleanup is missing its root type".to_owned(),
        )
    })?;
    provider.resolve_partial_binding(root_type, moved_fields)
}
