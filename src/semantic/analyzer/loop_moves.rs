use crate::ast::{Block, CaseBody, Expr, IfBranch, Stmt};

use super::super::errors::{SemanticError, SemanticErrorKind};
use super::Analyzer;

impl Analyzer {
    /// Records the ownership of every binding declared before a loop, so the
    /// back edges of its body can be compared against it (ADR-0016).
    pub(in crate::semantic::analyzer) fn enter_loop_ownership(&mut self) {
        let entry = self.model.bindings.iter().map(|binding| binding.ownership.clone()).collect();
        self.loop_entry_ownership.push(entry);
    }

    /// Checks the end of one loop-body iteration, unless the body always
    /// leaves through `break`, `continue` or `return` and never reaches it.
    pub(in crate::semantic::analyzer) fn leave_loop_ownership(
        &mut self,
        body: &Block,
        body_result: Result<(), SemanticError>,
    ) -> Result<(), SemanticError> {
        let result = body_result.and_then(|()| {
            if block_ends_iteration(body) {
                Ok(())
            } else {
                self.reject_moves_into_next_iteration()
            }
        });
        self.loop_entry_ownership.pop();
        result
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
