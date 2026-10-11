use crate::lexer::SourceSpan;
use crate::semantic::LoopExitKind;

use super::super::super::{BackendCleanupOrigin, BackendInstruction, BackendValueId};
use super::SharedExpressionProvider;

pub(super) fn emit_scope_cleanup(
    provider: &SharedExpressionProvider<'_>,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) {
    let returned_value = match instructions.last() {
        Some(BackendInstruction::Return { value, .. }) => *value,
        _ => None,
    };
    let mut cleanup = Vec::new();
    for plan in provider
        .cleanup_plans
        .iter()
        .filter(|plan| plan.span == span && matches!(plan.origin, BackendCleanupOrigin::Scope))
    {
        for (index, action) in plan.actions.iter().enumerate() {
            let super::super::super::BackendCleanupAction::DropBinding { name, .. } = action else {
                continue;
            };
            let Some(value) = local_value(provider, name) else { continue };
            if returned_value == Some(value) {
                continue;
            }
            let Some(descriptor) = plan.resolved_bridges.get(index).and_then(|items| items.first())
            else {
                continue;
            };
            cleanup.push(BackendInstruction::Cleanup {
                action: action.clone(),
                operand: Some(descriptor.operand(value)),
                span: plan.span,
            });
        }
    }
    let terminates = matches!(
        instructions.last(),
        Some(BackendInstruction::Return { .. })
            | Some(BackendInstruction::Fail { .. })
            | Some(BackendInstruction::Jump { .. })
            | Some(BackendInstruction::Branch { .. })
    );
    let insertion = instructions.len().saturating_sub(usize::from(terminates));
    instructions.splice(insertion..insertion, cleanup);
}

pub(super) fn append_loop_cleanup(
    provider: &SharedExpressionProvider<'_>,
    span: SourceSpan,
    kind: LoopExitKind,
    instructions: &mut Vec<BackendInstruction>,
) {
    for plan in provider.cleanup_plans.iter().filter(|plan| {
        matches!(
            &plan.origin,
            BackendCleanupOrigin::Loop { span: boundary, kind: expected }
                if *boundary == span && *expected == kind
        )
    }) {
        for (index, action) in plan.actions.iter().enumerate() {
            let super::super::super::BackendCleanupAction::DropBinding { name, .. } = action else {
                continue;
            };
            let Some(value) = binding_value(provider, name) else { continue };
            let Some(descriptor) = plan.resolved_bridges.get(index).and_then(|items| items.first())
            else {
                continue;
            };
            instructions.push(BackendInstruction::Cleanup {
                action: action.clone(),
                operand: Some(descriptor.operand(value)),
                span: plan.span,
            });
        }
    }
}

fn local_value(provider: &SharedExpressionProvider<'_>, name: &str) -> Option<BackendValueId> {
    provider.locals.iter().rev().find_map(|scope| scope.get(name).copied())
}

fn binding_value(provider: &SharedExpressionProvider<'_>, name: &str) -> Option<BackendValueId> {
    local_value(provider, name).or_else(|| provider.parameters.get(name).copied())
}
