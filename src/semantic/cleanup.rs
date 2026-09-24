use crate::ast::Role;
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::model::Binding;
use super::state::OwnershipState;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CleanupAction {
    EndBorrow { borrow_id: usize },
    DropPayloadField { binding_index: usize, enum_name: String, variant: String, field: String },
    DropBinding { binding_index: usize },
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
    actions.extend(binding_indices.iter().rev().filter_map(|index| {
        let binding = &bindings[*index];
        let owned = matches!(binding.role, Role::Erg | Role::Dat);
        let live = matches!(
            binding.ownership,
            OwnershipState::Active | OwnershipState::PartiallyMoved { .. }
        );
        (owned && live).then_some(CleanupAction::DropBinding { binding_index: *index })
    }));
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
                )
            })
            .collect();
        self.model.loop_unwind_plans.push(LoopUnwindPlan { span, kind, scopes: plans });
        Ok(())
    }
}
