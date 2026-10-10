use super::super::model::LoweredCleanupInstruction;
use super::{BackendCleanupPlan, BackendCleanupProvider, BackendContractError};

pub fn lower_semantic_cleanup_with_provider(
    model: &crate::semantic::SemanticModel,
    provider: &impl BackendCleanupProvider,
) -> Result<Vec<BackendCleanupPlan>, BackendContractError> {
    super::super::model::validate_cleanup_plans(model)
        .map_err(BackendContractError::InvalidCleanupContract)?;
    Ok(super::super::model::lower_cleanup_plans(model)
        .into_iter()
        .map(|plan| lower_plan(plan, model, provider))
        .collect::<Result<Vec<_>, _>>()?)
}

fn lower_plan(
    plan: super::super::model::LoweredCleanupPlan,
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
    moved_fields
        .iter()
        .filter_map(|path| match provider.resolve_field(root_type, path) {
            Ok(Some(descriptor)) => Some(Ok(descriptor)),
            Ok(None) => None,
            Err(error) => Some(Err(error)),
        })
        .collect()
}
