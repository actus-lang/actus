use crate::lexer::SourceSpan;

use super::super::super::{
    BackendCleanupAction, BackendCleanupOrigin, BackendContractError, BackendInstruction,
    BackendValueId,
};
use super::super::support::unsupported;
use super::SharedExpressionProvider;

pub(super) fn lower_binding(
    provider: &mut SharedExpressionProvider<'_>,
    name: &str,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(), BackendContractError> {
    let value = binding_value(provider, name)
        .ok_or_else(|| unsupported(span, "explicit drop binding is not lowered"))?;
    let Some(plan) = provider.cleanup_plans.iter().find(|plan| {
        matches!(plan.origin, BackendCleanupOrigin::Explicit { span: drop_span } if drop_span == span)
    }) else {
        return Err(unsupported(span, "explicit drop has no semantic cleanup plan"));
    };
    for (index, action) in plan.actions.iter().enumerate() {
        let action_name = match action {
            BackendCleanupAction::DropBinding { name, .. }
            | BackendCleanupAction::DropBindingFields { name, .. } => name,
            _ => continue,
        };
        if action_name != name {
            continue;
        }
        for descriptor in plan.resolved_bridges.get(index).into_iter().flatten() {
            if !super::cleanup::descriptor_applies(provider, value, descriptor) {
                continue;
            }
            instructions.push(BackendInstruction::Cleanup {
                action: action.clone(),
                operand: Some(descriptor.operand(value)),
                span,
            });
        }
    }
    Ok(())
}

fn binding_value(provider: &SharedExpressionProvider<'_>, name: &str) -> Option<BackendValueId> {
    provider
        .locals
        .iter()
        .rev()
        .find_map(|scope| scope.get(name).copied())
        .or_else(|| provider.parameters.get(name).copied())
}
