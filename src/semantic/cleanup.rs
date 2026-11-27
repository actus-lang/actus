use crate::ast::{Role, TypeName};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::model::Binding;
use super::state::OwnershipState;

/// One step of an exit list (ADR-0125). Every action names its binding by
/// binding index, never by name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CleanupAction {
    EndBorrow {
        borrow_id: usize,
    },
    /// Releases the payload field `field` of `variant` of the enum owner
    /// `binding_index`, which a `case dat` branch consumed without binding the
    /// field. The variant `_` and an empty field release the whole subject by
    /// its active variant (a wildcard branch of `case dat`).
    DropPayloadField {
        binding_index: usize,
        enum_name: String,
        variant: String,
        field: String,
    },
    /// Releases the whole value of a live owner.
    DropBinding {
        binding_index: usize,
    },
    /// Releases what a partially moved owner still owns at this exit: every
    /// owned field except the paths in `moved_fields` (ADR-0046). The owner's
    /// own drop contract does not run, because it no longer holds a whole value.
    DropRemainingFields {
        binding_index: usize,
        moved_fields: Vec<String>,
    },
    ResetArena {
        binding_index: usize,
    },
}

/// Where the exit lists of one analysed verb body lie in the model (ADR-0125).
/// The plans of a body are recorded while it is analysed, so each list of the
/// model holds them as one contiguous range. A body is identified by its name
/// and its source span: a specialised generic verb has a name of its own and
/// every other verb or performance method a span of its own. Code generation
/// looks an exit up only inside the ranges of the body it lowers, so equal
/// spans in two instances of one generic verb never meet.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerbCleanup {
    pub name: String,
    pub span: SourceSpan,
    /// Scope ends, in `SemanticModel::cleanup_plans`.
    pub scopes: std::ops::Range<usize>,
    /// `return` statements and `?` failure exits, in `return_unwind_plans`.
    pub returns: std::ops::Range<usize>,
    /// `break` and `continue` statements, in `loop_unwind_plans`.
    pub loops: std::ops::Range<usize>,
    /// Explicit `drop(x)` statements, in `explicit_drops`.
    pub drops: std::ops::Range<usize>,
    /// Assignments that release the value an owner held, in `replacement_drops`.
    pub replacements: std::ops::Range<usize>,
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
    // The state is the one at this exit (ADR-0125): a partially moved owner
    // releases the fields it still owns here, whatever happens to it later.
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
    actions.extend(live_owners.iter().map(|index| match &bindings[*index].ownership {
        OwnershipState::PartiallyMoved { fields } => CleanupAction::DropRemainingFields {
            binding_index: *index,
            moved_fields: fields.clone(),
        },
        _ => CleanupAction::DropBinding { binding_index: *index },
    }));
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
    /// Opens the record of the exit lists of the verb body `name` at `span`.
    pub(super) fn begin_verb_cleanup(&mut self, name: &str, span: SourceSpan) {
        let model = &self.model;
        let record = VerbCleanup {
            name: name.to_owned(),
            span,
            scopes: model.cleanup_plans.len()..model.cleanup_plans.len(),
            returns: model.return_unwind_plans.len()..model.return_unwind_plans.len(),
            loops: model.loop_unwind_plans.len()..model.loop_unwind_plans.len(),
            drops: model.explicit_drops.len()..model.explicit_drops.len(),
            replacements: model.replacement_drops.len()..model.replacement_drops.len(),
        };
        self.model.verb_cleanups.push(record);
    }

    /// Closes the record opened by `begin_verb_cleanup` after the body was
    /// analysed, so that it covers every exit list the body produced.
    pub(super) fn end_verb_cleanup(&mut self) {
        let (scopes, returns, loops, drops, replacements) = (
            self.model.cleanup_plans.len(),
            self.model.return_unwind_plans.len(),
            self.model.loop_unwind_plans.len(),
            self.model.explicit_drops.len(),
            self.model.replacement_drops.len(),
        );
        if let Some(record) = self.model.verb_cleanups.last_mut() {
            record.scopes.end = scopes;
            record.returns.end = returns;
            record.loops.end = loops;
            record.drops.end = drops;
            record.replacements.end = replacements;
        }
    }

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
