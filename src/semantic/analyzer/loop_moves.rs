use crate::ast::{Block, CaseBody, Expr, IfBranch, Stmt};
use crate::lexer::SourceSpan;

use super::super::errors::{SemanticError, SemanticErrorKind};
use super::Analyzer;

impl Analyzer {
    /// Records the ownership of every binding declared before a loop, so the
    /// back edges of its body can be compared against it (ADR-0016).
    pub(in crate::semantic::analyzer) fn enter_loop_ownership(&mut self) {
        let entry = self.model.bindings.iter().map(|binding| binding.ownership.clone()).collect();
        self.loop_entry_ownership.push(entry);
        self.loop_break_ownership.push(Vec::new());
    }

    /// Records the ownership of the bindings declared before the innermost
    /// loop at one of its `break` statements. A `break` belongs to its
    /// innermost loop only, and the state at a `return` is never recorded.
    pub(in crate::semantic::analyzer) fn record_loop_break(&mut self) {
        let Some(entry) = self.loop_entry_ownership.last() else { return };
        let state = self.model.bindings[..entry.len()]
            .iter()
            .map(|binding| binding.ownership.clone())
            .collect();
        if let Some(breaks) = self.loop_break_ownership.last_mut() {
            breaks.push(state);
        }
    }

    /// The state after a loop is the join of the states of every path that
    /// leaves it: each `break`, and for a `for` loop also its normal exit,
    /// which holds the state at the loop entry because the back edges are
    /// checked against it. A value moved or dropped before a `break` is moved
    /// or dropped after the loop, so it is neither used nor cleaned up again.
    /// Paths that leave a binding in different states are rejected as the
    /// branches of an `if` are. A loop without a `break` is only left by
    /// `return` (or, for `for`, its normal exit) and keeps its end state.
    fn join_loop_exits(
        &mut self,
        entry: Vec<crate::semantic::state::OwnershipState>,
        normal_exit: bool,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let mut breaks = self.loop_break_ownership.pop().unwrap_or_default();
        if breaks.is_empty() {
            return Ok(());
        }
        if normal_exit {
            breaks.insert(0, entry);
        }
        let Some(first) = breaks.first() else { return Ok(()) };
        for other in breaks.iter().skip(1) {
            for (index, (expected, found)) in first.iter().zip(other).enumerate() {
                if expected != found {
                    return Err(SemanticError {
                        kind: SemanticErrorKind::BranchStateMismatch {
                            name: self.model.bindings[index].name.clone(),
                            expected: format!("{expected:?}"),
                            found: format!("{found:?}"),
                        },
                        span,
                    });
                }
            }
        }
        for (binding, ownership) in self.model.bindings.iter_mut().zip(first) {
            binding.ownership = ownership.clone();
        }
        Ok(())
    }

    /// Checks the end of one loop-body iteration, unless the body always
    /// leaves through `break`, `continue` or `return` and never reaches it,
    /// then joins the states of the paths that leave the loop. `normal_exit`
    /// is true for a `for` loop, which can also end without a `break`.
    pub(in crate::semantic::analyzer) fn leave_loop_ownership(
        &mut self,
        body: &Block,
        normal_exit: bool,
        body_result: Result<(), SemanticError>,
    ) -> Result<(), SemanticError> {
        let result = body_result.and_then(|()| {
            if block_ends_iteration(body) {
                Ok(())
            } else {
                self.reject_moves_into_next_iteration()
            }
        });
        let entry = self.loop_entry_ownership.pop().unwrap_or_default();
        result.and_then(|()| self.join_loop_exits(entry, normal_exit, body.span))
    }

    /// A binding declared before the loop and live when the loop was entered
    /// must still be in that state wherever the body starts the next
    /// iteration: at a `continue` and at the end of the body. The body is
    /// analysed once, so a move that survives to such a point would otherwise
    /// be repeated by the next iteration and free the value twice. Values
    /// declared inside the body are new in every iteration and are not
    /// compared; a path that leaves the loop never reaches this check.
    pub(in crate::semantic::analyzer) fn reject_moves_into_next_iteration(
        &self,
    ) -> Result<(), SemanticError> {
        let Some(entry) = self.loop_entry_ownership.last() else { return Ok(()) };
        for (index, before) in entry.iter().enumerate() {
            let binding = &self.model.bindings[index];
            if before.is_live()
                && binding.ownership != *before
                && let Some(&span) = self.ownership_change_spans.get(&index)
            {
                return Err(SemanticError {
                    kind: SemanticErrorKind::MovedInPreviousIteration {
                        name: binding.name.clone(),
                    },
                    span,
                });
            }
        }
        Ok(())
    }
}

fn block_ends_iteration(block: &Block) -> bool {
    block.statements.last().is_some_and(|statement| {
        match statement {
        Stmt::Return { .. } | Stmt::Break { .. } | Stmt::Continue { .. } => true,
        Stmt::Block(inner) => block_ends_iteration(inner),
        Stmt::If { then_branch, else_branch, .. } => {
            if_ends_iteration(then_branch, else_branch.as_ref())
        }
        Stmt::Expression { expression: Expr::Case { branches, .. }, .. } => {
            !branches.is_empty()
                && branches.iter().all(|branch| {
                    matches!(&branch.body, CaseBody::Block(block) if block_ends_iteration(block))
                })
        }
        _ => false,
    }
    })
}

fn if_ends_iteration(then_branch: &Block, else_branch: Option<&IfBranch>) -> bool {
    block_ends_iteration(then_branch)
        && match else_branch {
            Some(IfBranch::Block(block)) => block_ends_iteration(block),
            Some(IfBranch::ElseIf(expression)) => match expression.as_ref() {
                Expr::If { then_branch, else_branch, .. } => {
                    if_ends_iteration(then_branch, else_branch.as_ref())
                }
                _ => false,
            },
            None => false,
        }
}
