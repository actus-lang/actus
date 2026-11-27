//! The executor of the analyzer's exit lists (ADR-0125). Each exit of a verb
//! runs exactly the list the analyzer computed for it, with the values the
//! lowering holds at that point; nothing here decides what is live.

use crate::lexer::SourceSpan;
use crate::semantic::LoopExitKind;

use super::super::super::{BackendInstruction, BackendLayout, BackendValueId, BackendValueType};
use super::super::errors::unsupported;
use super::super::exits::{CleanupGuard, OwnerRelease, ScopeRelease};
use super::SharedExpressionProvider;
use crate::codegen::{BackendContractError, BackendExpressionProvider};

/// Whether a step that releases an owner's whole value (one without a field)
/// fits the representation the lowering holds for it: a buffer or region
/// handle, or a large aggregate whose storage the lowering allocated on the
/// heap. A value held in place has no storage of its own to release.
fn descriptor_applies(
    provider: &SharedExpressionProvider<'_>,
    value: BackendValueId,
    descriptor: &super::super::super::BackendCleanupResolvedDescriptor,
) -> bool {
    let bridge = descriptor.descriptor.bridge;
    if matches!(bridge, super::super::super::BackendCleanupBridge::BufferDrop) {
        if descriptor.field.is_some() {
            return true;
        }
        return provider.value_type(value) == Some(super::super::super::BackendValueType::Pointer)
            && provider.nominal_type(value).is_some_and(|type_name| type_name.name == "Buffer");
    }
    if matches!(bridge, super::super::super::BackendCleanupBridge::RegionDrop) {
        if descriptor.field.is_some() {
            return true;
        }
        return match provider.value_type(value) {
            Some(super::super::super::BackendValueType::Region { .. }) => {
                provider.nominal_type(value).is_some_and(|type_name| type_name.name == "Region")
            }
            Some(super::super::super::BackendValueType::Pointer) => {
                provider.nominal_type(value).is_some_and(|type_name| type_name.name == "Region")
            }
            _ => false,
        };
    }
    if !matches!(bridge, super::super::super::BackendCleanupBridge::AggregateDrop { .. }) {
        return true;
    }
    if descriptor.field.is_none() && !provider.is_heap_owned_aggregate(value) {
        return false;
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

/// Runs the end of the scope at `span` on its fallthrough path.
pub(super) fn emit_scope_end(
    provider: &mut SharedExpressionProvider<'_>,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) {
    if let Some(scope) = provider.exits.and_then(|exits| exits.scope_end(span)) {
        emit_scope_release(provider, scope, instructions);
    }
}

/// Runs the list of the `return` (or `?` failure exit) at `span`: every scope
/// it leaves, innermost first. A scope that the lowering opened inside the
/// returned expression itself, an arm of `return if` or `return case`, ended
/// before the analyzer reached the `return`, so it is closed with its own
/// scope end first. An exit without a list is an internal error: the
/// analyzer plans every exit of an accepted program.
pub(super) fn emit_return_exit(
    provider: &mut SharedExpressionProvider<'_>,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(), BackendContractError> {
    let Some(exits) = provider.exits else { return Ok(()) };
    let planned = exits.return_exit(span).ok_or_else(|| {
        unsupported(
            span,
            format!(
                "internal compiler error: a return of `{}` has no semantic cleanup list",
                provider.verb_name
            ),
        )
    })?;
    let mut released = planned.iter().map(|scope| scope.span).collect::<Vec<_>>();
    for open in provider.scope_spans.clone().into_iter().rev() {
        if released.contains(&open) {
            continue;
        }
        released.push(open);
        if let Some(scope) = exits.scope_end(open) {
            emit_scope_release(provider, scope, instructions);
        }
    }
    for scope in planned {
        emit_scope_release(provider, scope, instructions);
    }
    Ok(())
}

/// Runs the list of the `break` or `continue` at `span`: the scopes up to the
/// loop boundary, innermost first.
pub(super) fn emit_loop_exit(
    provider: &mut SharedExpressionProvider<'_>,
    span: SourceSpan,
    kind: LoopExitKind,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(), BackendContractError> {
    let Some(exits) = provider.exits else { return Ok(()) };
    let scopes = exits.loop_exit(span, &kind).ok_or_else(|| {
        unsupported(
            span,
            format!(
                "internal compiler error: a loop exit of `{}` has no semantic cleanup list",
                provider.verb_name
            ),
        )
    })?;
    for scope in scopes {
        emit_scope_release(provider, scope, instructions);
    }
    Ok(())
}

/// Runs the explicit `drop` at `span`. A drop of an owner that holds no
/// resource releases nothing and has no list of its own.
pub(super) fn emit_explicit_drop(
    provider: &mut SharedExpressionProvider<'_>,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) {
    if let Some(owner) = provider.exits.and_then(|exits| exits.explicit_drop(span)) {
        emit_owner_release(provider, owner, span, instructions);
    }
}

/// Releases the value an owner holds before the assignment at `span` stores
/// a new one; it runs before the owner's value is replaced.
pub(super) fn emit_replacement(
    provider: &mut SharedExpressionProvider<'_>,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) {
    if let Some(owner) = provider.exits.and_then(|exits| exits.replacement(span)) {
        emit_owner_release(provider, owner, span, instructions);
    }
}

fn emit_scope_release(
    provider: &mut SharedExpressionProvider<'_>,
    scope: &ScopeRelease,
    instructions: &mut Vec<BackendInstruction>,
) {
    for owner in &scope.owners {
        emit_owner_release(provider, owner, scope.span, instructions);
    }
}

/// Emits the steps that release one owner, addressed from the value the
/// lowering holds for it now. A step on a payload runs only for the variant
/// that owns it. The binding is looked up by the name it was declared with:
/// the analyzer rejects shadowing, so among the open scopes the name denotes
/// exactly the binding the list names by index.
fn emit_owner_release(
    provider: &mut SharedExpressionProvider<'_>,
    owner: &OwnerRelease,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) {
    let Some(value) = provider.binding_value(&owner.name) else { return };
    for step in &owner.steps {
        if step.descriptor.field.is_none() && !descriptor_applies(provider, value, &step.descriptor)
        {
            continue;
        }
        let operand = step.descriptor.operand(value);
        if step.guards.is_empty() {
            instructions.push(BackendInstruction::Cleanup {
                action: owner.action.clone(),
                operand: Some(operand),
                span,
            });
            continue;
        }
        let condition = guard_condition(
            provider,
            value,
            step.descriptor.layout,
            &step.guards,
            span,
            instructions,
        );
        instructions.push(BackendInstruction::CleanupWhen {
            condition,
            action: owner.action.clone(),
            operand,
            span,
        });
    }
}

/// Loads each variant tag a step depends on from the owner's value and
/// yields true only when every one of them selects the step's variant.
fn guard_condition(
    provider: &mut SharedExpressionProvider<'_>,
    value: BackendValueId,
    root_layout: BackendLayout,
    guards: &[CleanupGuard],
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> BackendValueId {
    let mut condition: Option<BackendValueId> = None;
    for guard in guards {
        let ty = BackendValueType::Integer { signed: false, width: guard.size * 8 };
        let size = u32::from(guard.size);
        let tag = provider.allocate_value(ty);
        instructions.push(BackendInstruction::AggregateFieldLoad {
            result: tag,
            base: value,
            aggregate_layout: root_layout,
            offset: guard.offset,
            field_layout: BackendLayout { size, alignment: size },
            ty,
            span,
        });
        let expected = provider.allocate_value(ty);
        let discriminant = u64::from(guard.discriminant);
        instructions.push(BackendInstruction::Constant {
            result: expected,
            ty,
            value: discriminant,
            span,
        });
        let matched = provider.allocate_value(BackendValueType::Bool);
        instructions.push(BackendInstruction::IntegerCompare {
            result: matched,
            operation: super::super::super::BackendIntegerCompareOp::Equals,
            left: tag,
            right: expected,
            signed: false,
            span,
        });
        condition = Some(match condition {
            None => matched,
            Some(previous) => {
                let both = provider.allocate_value(BackendValueType::Bool);
                instructions.push(BackendInstruction::ScalarSelect {
                    result: both,
                    condition: previous,
                    then_value: matched,
                    else_value: previous,
                    ty: BackendValueType::Bool,
                    span,
                });
                both
            }
        });
    }
    condition.expect("a guarded cleanup step has at least one guard")
}
