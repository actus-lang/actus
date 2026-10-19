use crate::lexer::SourceSpan;
use crate::semantic::LoopExitKind;

use super::super::super::{BackendCleanupOrigin, BackendInstruction, BackendValueId};
use super::SharedExpressionProvider;

pub(crate) fn descriptor_applies(
    provider: &SharedExpressionProvider<'_>,
    value: BackendValueId,
    descriptor: &super::super::super::BackendCleanupResolvedDescriptor,
) -> bool {
    let bridge = descriptor.descriptor.bridge;
    if matches!(bridge, super::super::super::BackendCleanupBridge::BufferDrop) {
        return provider.value_type(value) == Some(super::super::super::BackendValueType::Pointer)
            && provider.nominal_type(value).is_some_and(|type_name| type_name.name == "Buffer");
    }
    if matches!(bridge, super::super::super::BackendCleanupBridge::RegionDrop) {
        return provider.value_type(value) == Some(super::super::super::BackendValueType::Pointer)
            && provider.nominal_type(value).is_some_and(|type_name| type_name.name == "Region");
    }
    if !matches!(bridge, super::super::super::BackendCleanupBridge::AggregateDrop { .. }) {
        return true;
    }
    let actual = match provider.value_type(value) {
        Some(super::super::super::BackendValueType::Aggregate { layout })
        | Some(super::super::super::BackendValueType::Region { layout }) => Some(layout),
        Some(super::super::super::BackendValueType::Pointer) => provider
            .nominal_type(value)
            .and_then(|type_name| provider.layouts.layout_for(type_name)),
        _ => None,
    };
    actual == Some(descriptor.layout)
}

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
            let name = match action {
                super::super::super::BackendCleanupAction::DropBinding { name, .. }
                | super::super::super::BackendCleanupAction::DropBindingFields { name, .. } => name,
                _ => continue,
            };
            let Some(value) = local_value(provider, name) else { continue };
            if returned_value == Some(value) {
                continue;
            }
            if let Some(descriptors) = plan.resolved_bridges.get(index) {
                for descriptor in descriptors {
                    if !descriptor_applies(provider, value, descriptor) {
                        continue;
                    }
                    cleanup.push(BackendInstruction::Cleanup {
                        action: action.clone(),
                        operand: Some(descriptor.operand(value)),
                        span: plan.span,
                    });
                }
            }
        }
    }
    if matches!(instructions.last(), Some(BackendInstruction::Branch { .. })) {
        return;
    }
    let terminates = matches!(
        instructions.last(),
        Some(BackendInstruction::Return { .. })
            | Some(BackendInstruction::Fail { .. })
            | Some(BackendInstruction::Jump { .. })
    );
    let insertion = instructions.len().saturating_sub(usize::from(terminates));
    instructions.splice(insertion..insertion, cleanup);
}

pub(super) fn emit_replacement_cleanup(
    provider: &SharedExpressionProvider<'_>,
    name: &str,
    value: super::super::super::BackendValueId,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) {
    for plan in provider.cleanup_plans.iter().filter(|plan| {
        matches!(plan.origin, BackendCleanupOrigin::Scope)
            && plan.actions.iter().enumerate().any(|(index, action)| {
                let action_name = match action {
                    super::super::super::BackendCleanupAction::DropBinding { name, .. }
                    | super::super::super::BackendCleanupAction::DropBindingFields {
                        name, ..
                    } => name,
                    _ => return false,
                };
                action_name == name
                    && plan.resolved_bridges.get(index).is_some_and(|items| !items.is_empty())
            })
    }) {
        for (index, action) in plan.actions.iter().enumerate() {
            let action_name = match action {
                super::super::super::BackendCleanupAction::DropBinding { name, .. }
                | super::super::super::BackendCleanupAction::DropBindingFields { name, .. } => name,
                _ => continue,
            };
            if action_name != name {
                continue;
            }
            for descriptor in plan.resolved_bridges.get(index).into_iter().flatten() {
                if descriptor_applies(provider, value, descriptor) {
                    instructions.push(BackendInstruction::Cleanup {
                        action: action.clone(),
                        operand: Some(descriptor.operand(value)),
                        span,
                    });
                }
            }
        }
    }
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
            let name = match action {
                super::super::super::BackendCleanupAction::DropBinding { name, .. }
                | super::super::super::BackendCleanupAction::DropBindingFields { name, .. } => name,
                _ => continue,
            };
            let Some(value) = binding_value(provider, name) else { continue };
            if let Some(descriptors) = plan.resolved_bridges.get(index) {
                for descriptor in descriptors {
                    if !descriptor_applies(provider, value, descriptor) {
                        continue;
                    }
                    instructions.push(BackendInstruction::Cleanup {
                        action: action.clone(),
                        operand: Some(descriptor.operand(value)),
                        span: plan.span,
                    });
                }
            }
        }
    }
}

fn local_value(provider: &SharedExpressionProvider<'_>, name: &str) -> Option<BackendValueId> {
    provider.locals.last().and_then(|scope| scope.get(name).copied())
}

fn binding_value(provider: &SharedExpressionProvider<'_>, name: &str) -> Option<BackendValueId> {
    local_value(provider, name).or_else(|| provider.parameters.get(name).copied())
}
