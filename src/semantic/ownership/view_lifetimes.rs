//! A view held by a value belongs to the place that holds the value
//! (ADR-0046 2.3). This module decides how long the source of a view must
//! live when the value is stored in a binding, a field or an element, which
//! views an `if` or `case` value and a `case` pattern variable hold, and
//! which sources a branch join must keep frozen.

use crate::ast::{Block, CaseBody, Expr, IfBranch, Role, Stmt, TypeName};
use crate::lexer::SourceSpan;

use crate::semantic::analyzer::{Analyzer, expression_span};
use crate::semantic::errors::{SemanticError, SemanticErrorKind};
use crate::semantic::model::{Origin, OriginRoot};
use crate::semantic::origins::{combine, derive_from};
use crate::semantic::state::AccessState;

impl Analyzer {
    /// Freezes every source of the views that a value holds until the end of
    /// the scope of the binding that now holds the value (ADR-0046 2.3), with
    /// the diagnostics of a direct construction. A source declared in a scope
    /// deeper than the binding's is released before the binding, so storing a
    /// view of it is rejected as an escaping borrow (`E1018`) at `span`.
    /// Views of `abs` parameters and views with no tracked source freeze
    /// nothing here.
    pub(in crate::semantic) fn hold_views_in_binding(
        &mut self,
        destination: usize,
        origins: &[Origin],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let destination_depth = self.binding_scope_depth.get(&destination).copied().unwrap_or(1);
        for owner in view_owner_indices(origins) {
            if owner == destination {
                continue;
            }
            let name = self.model.bindings[owner].name.clone();
            if self.binding_scope_depth.get(&owner).copied().unwrap_or(1) > destination_depth {
                return Err(SemanticError {
                    kind: SemanticErrorKind::BorrowedReturn { name },
                    span,
                });
            }
            if self.owner_frozen_through(owner, destination_depth) {
                continue;
            }
            self.ensure_readable(owner, &name, span)?;
            let borrow_id = self.add_borrow_at(destination_depth, &name, None, owner, span);
            self.holder_borrow_ids.insert(borrow_id);
        }
        Ok(())
    }

    /// Whether an active borrow of `owner` already lasts to the end of the
    /// scope at `depth`, so the source needs no second borrow.
    fn owner_frozen_through(&self, owner: usize, depth: usize) -> bool {
        self.model.borrows.iter().any(|record| {
            self.active_borrow_ids.contains(&record.id)
                && record.scope_depth <= depth
                && self.borrow_owner_indices.get(&record.id) == Some(&owner)
        })
    }

    /// Records the views that a value stored in an element adds to the
    /// binding that owns the array, like a field store does. Storing views
    /// through an `ins` parameter would let them outlive the verb through the
    /// caller's object, so only views of `abs` parameters may be stored there.
    pub(in crate::semantic) fn record_element_view_provenance(
        &mut self,
        target: &Expr,
        value: &Expr,
    ) -> Result<(), SemanticError> {
        let origins = self.value_view_origins(value);
        let Some(index) = self.root_binding_index(target).filter(|_| !origins.is_empty()) else {
            return Ok(());
        };
        if self.model.bindings[index].role == Role::Ins {
            self.reject_escaping_views(value)?;
        }
        self.hold_views_in_binding(index, &origins, expression_span(value))?;
        self.binding_held_views.entry(index).or_default().extend(origins);
        Ok(())
    }

    /// The views that the value of one arm of an `if` or `case` holds: the
    /// value of a block is its unterminated tail, which may itself be an `if`
    /// statement whose branches carry the value.
    pub(in crate::semantic) fn block_tail_view_origins(&self, block: &Block) -> Vec<Origin> {
        match block.statements.last() {
            Some(Stmt::Expression { expression, span })
                if span.end == expression_span(expression).end =>
            {
                self.value_view_origins(expression)
            }
            Some(Stmt::If { then_branch, else_branch, .. }) => {
                let mut origins = self.block_tail_view_origins(then_branch);
                origins.extend(match else_branch {
                    Some(IfBranch::Block(block)) => self.block_tail_view_origins(block),
                    Some(IfBranch::ElseIf(nested)) => self.value_view_origins(nested),
                    None => Vec::new(),
                });
                origins
            }
            _ => Vec::new(),
        }
    }

    /// The views that the value of a `case` arm holds.
    pub(in crate::semantic) fn case_body_view_origins(&self, body: &CaseBody) -> Vec<Origin> {
        match body {
            CaseBody::Expression(expression) => self.value_view_origins(expression),
            CaseBody::Block(block) => self.block_tail_view_origins(block),
        }
    }

    /// Stores the views that the value of the `if` or `case` at `span` holds:
    /// the union of the views of every arm, read while the arm was live.
    pub(in crate::semantic) fn record_conditional_views(
        &mut self,
        span: SourceSpan,
        origins: Vec<Origin>,
    ) {
        let key = self.expression_cache_key(span);
        self.conditional_view_origins.insert(key, origins);
    }

    /// The views of the `if` or `case` at `span`, as stored after its arms.
    pub(in crate::semantic) fn conditional_views(&self, span: SourceSpan) -> Vec<Origin> {
        self.conditional_view_origins
            .get(&self.expression_cache_key(span))
            .cloned()
            .unwrap_or_default()
    }

    /// A variable bound by a `case` pattern holds the views of the payload of
    /// its subject: the views the subject holds, or a view of the subject
    /// itself when it holds none, so a variable bound from an `abs` parameter
    /// derives from that parameter (ADR-0046 2.3). A `dat` variable is an
    /// owner, so it holds the views but is not a view of the subject.
    pub(in crate::semantic) fn record_pattern_view_provenance(
        &mut self,
        index: usize,
        payload: &TypeName,
        mode: crate::ast::CaseMode,
        subject: &Expr,
    ) {
        let held = self.value_view_origins(subject);
        let inherited = if held.is_empty() {
            vec![derive_from(self.origin_of(subject))]
                .into_iter()
                .filter(|origin| *origin != Origin::None)
                .collect()
        } else {
            held
        };
        if inherited.is_empty() {
            return;
        }
        if self.type_holds_view(payload) {
            self.binding_held_views.insert(index, inherited.clone());
        }
        if mode != crate::ast::CaseMode::Dat {
            self.binding_origins.insert(index, combine(inherited));
        }
    }

    /// Ends the holder borrows registered since `watermark` (a value of
    /// `next_borrow_id` taken when a branch or arm started) by a branch that
    /// leaves the verb or the loop. The path that continues after the
    /// conditional never stored the view, so its sources are released unless
    /// another active borrow still holds them.
    pub(in crate::semantic) fn end_leaving_holder_borrows(&mut self, watermark: usize) {
        let ended = self
            .holder_borrow_ids
            .iter()
            .copied()
            .filter(|id| *id >= watermark)
            .collect::<Vec<_>>();
        for borrow_id in ended {
            self.holder_borrow_ids.remove(&borrow_id);
            self.active_borrow_ids.remove(&borrow_id);
            let Some(owner) = self.borrow_owner_indices.get(&borrow_id).copied() else {
                continue;
            };
            let still_borrowed = self
                .active_borrow_ids
                .iter()
                .any(|other| self.borrow_owner_indices.get(other) == Some(&owner));
            if !still_borrowed {
                self.model.bindings[owner].access = AccessState::Mutable;
            }
        }
    }

    /// Gives back the freezes of the views that were stored in a binding of
    /// an enclosing scope inside a branch. The branch states were restored
    /// before the join, and the freeze belongs to the binding, not to the
    /// branch that stored the view.
    pub(in crate::semantic) fn refreeze_holder_borrows(&mut self) {
        let mut active = self
            .holder_borrow_ids
            .iter()
            .copied()
            .filter(|id| self.active_borrow_ids.contains(id))
            .collect::<Vec<_>>();
        active.sort_unstable();
        for borrow_id in active {
            let Some(owner) = self.borrow_owner_indices.get(&borrow_id).copied() else {
                continue;
            };
            match &mut self.model.bindings[owner].access {
                AccessState::Mutable => {
                    self.model.bindings[owner].access =
                        AccessState::Frozen { borrow_ids: vec![borrow_id] };
                }
                AccessState::Frozen { borrow_ids } if !borrow_ids.contains(&borrow_id) => {
                    borrow_ids.push(borrow_id);
                }
                _ => {}
            }
        }
    }

    /// Whether two branch states of a binding differ only by the freezes that
    /// views stored in a binding of an enclosing scope put on it
    /// (`holder_borrow_ids`), which belong to the binding, not to a branch, or
    /// by borrows that already ended with the branch's own scope.
    pub(in crate::semantic) fn same_branch_access(
        &self,
        left: &AccessState,
        right: &AccessState,
    ) -> bool {
        self.without_holder_borrows(left) == self.without_holder_borrows(right)
    }

    fn without_holder_borrows(&self, access: &AccessState) -> AccessState {
        let AccessState::Frozen { borrow_ids } = access else { return access.clone() };
        let kept = borrow_ids
            .iter()
            .copied()
            .filter(|id| {
                self.active_borrow_ids.contains(id) && !self.holder_borrow_ids.contains(id)
            })
            .collect::<Vec<_>>();
        if kept.is_empty() {
            AccessState::Mutable
        } else {
            AccessState::Frozen { borrow_ids: kept }
        }
    }
}

/// The distinct bindings whose values the origins name, in order.
fn view_owner_indices(origins: &[Origin]) -> Vec<usize> {
    let mut owners = Vec::new();
    for origin in origins {
        let roots = match origin {
            Origin::Binding { binding_index } => vec![*binding_index],
            Origin::Multiple { roots } => roots
                .iter()
                .filter_map(|root| match root {
                    OriginRoot::Binding(index) => Some(*index),
                    OriginRoot::Parameter(_) => None,
                })
                .collect(),
            _ => Vec::new(),
        };
        for index in roots {
            if !owners.contains(&index) {
                owners.push(index);
            }
        }
    }
    owners
}
