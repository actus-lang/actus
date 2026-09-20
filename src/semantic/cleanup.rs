use crate::ast::Role;

use super::analyzer::Analyzer;
use super::model::{Binding, BindingState};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CleanupAction {
    EndBorrow { borrow_id: usize },
    DropBinding { binding_index: usize },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScopeCleanup {
    pub depth: usize,
    pub actions: Vec<CleanupAction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnwindPlan {
    pub scopes: Vec<ScopeCleanup>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoopExitKind {
    Break,
    Continue,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopUnwindPlan {
    pub kind: LoopExitKind,
    pub scopes: Vec<ScopeCleanup>,
}

pub(super) fn plan_scope_cleanup(
    depth: usize,
    binding_indices: &[usize],
    borrow_ids: &[usize],
    bindings: &[Binding],
) -> ScopeCleanup {
    let mut actions = borrow_ids
        .iter()
        .rev()
        .map(|borrow_id| CleanupAction::EndBorrow { borrow_id: *borrow_id })
        .collect::<Vec<_>>();
    actions.extend(binding_indices.iter().rev().filter_map(|index| {
        let binding = &bindings[*index];
        let owned = matches!(binding.role, Role::Erg | Role::Dat);
        let live = matches!(binding.state, BindingState::Active | BindingState::Frozen { .. });
        (owned && live).then_some(CleanupAction::DropBinding { binding_index: *index })
    }));
    ScopeCleanup { depth, actions }
}

impl Analyzer {
    pub(super) fn plan_return_unwind(&mut self) {
        let plans = self
            .scopes
            .iter()
            .enumerate()
            .rev()
            .map(|(index, frame)| {
                plan_scope_cleanup(
                    index + 1,
                    &frame.declaration_indices,
                    &frame.borrow_ids,
                    &self.model.bindings,
                )
            })
            .collect();
        self.model.return_unwind_plans.push(UnwindPlan { scopes: plans });
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
                    &frame.declaration_indices,
                    &frame.borrow_ids,
                    &self.model.bindings,
                )
            })
            .collect();
        self.model.loop_unwind_plans.push(LoopUnwindPlan { kind, scopes: plans });
        Ok(())
    }
}
