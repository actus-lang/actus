use crate::ast::{Role, TypeName};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::model::Binding;
use super::state::OwnershipState;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CleanupAction {
    EndBorrow { borrow_id: usize },
    DropPayloadField { binding_index: usize, enum_name: String, variant: String, field: String },
    DropBinding { binding_index: usize },
    ResetArena { binding_index: usize },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScopeCleanup {
    pub depth: usize,
    pub span: SourceSpan,
    pub actions: Vec<CleanupAction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnwindPlan {
    pub span: SourceSpan,
    pub scopes: Vec<ScopeCleanup>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoopExitKind {
    Break,
    Continue,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopUnwindPlan {
    pub span: SourceSpan,
    pub kind: LoopExitKind,
    pub scopes: Vec<ScopeCleanup>,
}

pub(super) fn plan_scope_cleanup(
    depth: usize,
    span: SourceSpan,
    binding_indices: &[usize],
    borrow_ids: &[usize],
    payload_cleanup: &[(usize, String, String, String)],
    bindings: &[Binding],
    binding_type_names: &std::collections::HashMap<usize, TypeName>,
) -> ScopeCleanup {
    let mut actions = borrow_ids
        .iter()
        .rev()
        .map(|borrow_id| CleanupAction::EndBorrow { borrow_id: *borrow_id })
        .collect::<Vec<_>>();
    actions.extend(payload_cleanup.iter().rev().map(
        |(binding_index, enum_name, variant, field)| CleanupAction::DropPayloadField {
            binding_index: *binding_index,
            enum_name: enum_name.clone(),
            variant: variant.clone(),
            field: field.clone(),
        },
    ));
    // Only the live owner of a resource is cleaned up, exactly once (ADR-0027).
    // A moved or dropped owner and an `abs` or `ins` view hand the resource to
    // someone else, so neither a drop nor an arena reset is planned for them.
    let live_owners = binding_indices
        .iter()
        .rev()
        .copied()
        .filter(|index| {
            let binding = &bindings[*index];
            matches!(binding.role, Role::Erg | Role::Dat)
                && matches!(
                    binding.ownership,
                    OwnershipState::Active | OwnershipState::PartiallyMoved { .. }
                )
        })
        .collect::<Vec<_>>();
    actions.extend(
        live_owners.iter().map(|index| CleanupAction::DropBinding { binding_index: *index }),
    );
    actions.extend(
        live_owners
            .iter()
            .filter(|index| {
                binding_type_names.get(*index).is_some_and(|type_name| type_name.name == "Arena")
            })
            .map(|index| CleanupAction::ResetArena { binding_index: *index }),
    );
    ScopeCleanup { depth, span, actions }
}

impl Analyzer {
    pub(super) fn plan_return_unwind(&mut self, span: SourceSpan) {
        let plans = self
            .scopes
            .iter()
            .enumerate()
            .rev()
            .map(|(index, frame)| {
                plan_scope_cleanup(
                    index + 1,
                    frame.span,
                    &frame.declaration_indices,
                    &frame.borrow_ids,
                    &frame.payload_cleanup,
                    &self.model.bindings,
                    &self.binding_type_names,
                )
            })
            .collect();
        self.model.return_unwind_plans.push(UnwindPlan { span, scopes: plans });
    }

    pub(super) fn plan_loop_unwind(
        &mut self,
        kind: LoopExitKind,
        keyword: &str,
        span: crate::lexer::SourceSpan,
    ) -> Result<(), super::errors::SemanticError> {
        let Some(&boundary) = self.loop_boundaries.last() else {
            return Err(super::errors::SemanticError {
                kind: super::errors::SemanticErrorKind::LoopControlOutsideLoop {
                    keyword: keyword.to_owned(),
                },
                span,
            });
        };
        let plans = self.scopes[boundary..]
            .iter()
            .enumerate()
            .rev()
            .map(|(offset, frame)| {
                plan_scope_cleanup(
                    boundary + offset + 1,
                    frame.span,
                    &frame.declaration_indices,
                    &frame.borrow_ids,
                    &frame.payload_cleanup,
                    &self.model.bindings,
                    &self.binding_type_names,
                )
            })
            .collect();
        self.model.loop_unwind_plans.push(LoopUnwindPlan { span, kind, scopes: plans });
        Ok(())
    }
}
