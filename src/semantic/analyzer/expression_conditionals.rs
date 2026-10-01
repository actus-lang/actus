use crate::ast::{Expr, IfBranch, Stmt};

use super::super::errors::{SemanticError, SemanticErrorKind};
use super::Analyzer;
use super::validation::expression_span;

impl Analyzer {
    pub(super) fn visit_if_expression(&mut self, expression: &Expr) -> Result<(), SemanticError> {
        let Expr::If { condition, then_branch, else_branch, .. } = expression else {
            unreachable!("conditional visitor received a non-conditional expression")
        };
        self.visit_expression(condition)?;
        let condition_type =
            self.expression_type_name(condition).unwrap_or_else(|| "unknown".to_owned());
        if condition_type != "Bool" {
            return Err(SemanticError {
                kind: SemanticErrorKind::GuardTypeMismatch { found: condition_type },
                span: expression_span(condition),
            });
        }
        let initial_state = self.snapshot_binding_states();
        let binding_count = initial_state.len();
        self.visit_block(then_branch)?;
        let then_state = self.snapshot_binding_prefix(binding_count);
        self.restore_binding_states(&initial_state);
        if let Some(else_branch) = else_branch {
            match else_branch {
                IfBranch::Block(block) => self.visit_block(block)?,
                IfBranch::ElseIf(expression) => self.visit_expression(expression)?,
            }
        }
        let else_state = self.snapshot_binding_prefix(binding_count);
        self.join_if_states(
            &initial_state,
            &then_state,
            &else_state,
            then_branch,
            else_branch.as_ref(),
            expression,
        )?;
        self.validate_if_result(then_branch, else_branch.as_ref(), expression)?;
        self.model.conditional_facts.push(super::super::model::ConditionalFact {
            span: expression_span(expression),
            condition_span: expression_span(condition),
            condition_type,
            then_type: block_tail_type(self, then_branch),
            else_type: else_branch.as_ref().and_then(|branch| match branch {
                IfBranch::Block(block) => block_tail_type(self, block),
                IfBranch::ElseIf(expression) => self.expression_type_name(expression),
            }),
        });
        Ok(())
    }

    fn join_if_states(
        &mut self,
        initial_state: &[(super::super::state::OwnershipState, super::super::state::AccessState)],
        then_state: &[(super::super::state::OwnershipState, super::super::state::AccessState)],
        else_state: &[(super::super::state::OwnershipState, super::super::state::AccessState)],
        then_branch: &crate::ast::Block,
        else_branch: Option<&IfBranch>,
        expression: &Expr,
    ) -> Result<(), SemanticError> {
        let then_diverges = branch_diverges(then_branch);
        let else_diverges = else_branch.is_some_and(|branch| else_branch_diverges(self, branch));
        match (then_diverges, else_diverges) {
            (true, false) => self.restore_binding_states(else_state),
            (false, true) => self.restore_binding_states(then_state),
            (true, true) => self.restore_binding_states(initial_state),
            (false, false) => {
                self.validate_branch_join(
                    &[then_state.to_vec(), else_state.to_vec()],
                    expression_span(expression),
                )?;
                self.restore_binding_states(then_state);
            }
        }
        Ok(())
    }

    fn validate_if_result(
        &self,
        then_branch: &crate::ast::Block,
        else_branch: Option<&IfBranch>,
        expression: &Expr,
    ) -> Result<(), SemanticError> {
        let Some(else_branch) = else_branch else {
            if self.expected_expression_type.is_some() {
                return Err(self.if_type_mismatch("value", "Void", expression));
            }
            return Ok(());
        };
        let then_type = block_tail_type(self, then_branch);
        let else_type = match else_branch {
            IfBranch::Block(block) => block_tail_type(self, block),
            IfBranch::ElseIf(nested) => self.expression_type_name(nested),
        };
        if then_type == else_type || (then_type.is_none() && else_type.is_none()) {
            return Ok(());
        }
        if then_type.is_none() && branch_diverges(then_branch) {
            return Ok(());
        }
        if else_type.is_none() && else_branch_diverges(self, else_branch) {
            return Ok(());
        }
        Err(self.if_type_mismatch(
            then_type.as_deref().unwrap_or("Void"),
            else_type.as_deref().unwrap_or("Void"),
            expression,
        ))
    }

    fn if_type_mismatch(&self, expected: &str, found: &str, expression: &Expr) -> SemanticError {
        SemanticError {
            kind: SemanticErrorKind::TypeMismatch {
                callee: "if".to_owned(),
                parameter: "branches".to_owned(),
                expected: expected.to_owned(),
                found: found.to_owned(),
            },
            span: expression_span(expression),
        }
    }
}

fn block_tail_type(analyzer: &Analyzer, block: &crate::ast::Block) -> Option<String> {
    let Some(Stmt::Expression { expression, span }) = block.statements.last() else {
        return None;
    };
    (span.end == expression_span(expression).end)
        .then(|| analyzer.expression_type_name(expression))?
}

fn branch_diverges(block: &crate::ast::Block) -> bool {
    matches!(
        block.statements.last(),
        Some(Stmt::Return { .. } | Stmt::Break { .. } | Stmt::Continue { .. })
    )
}

fn else_branch_diverges(analyzer: &Analyzer, branch: &IfBranch) -> bool {
    match branch {
        IfBranch::Block(block) => branch_diverges(block),
        IfBranch::ElseIf(expression) => analyzer.expression_type_name(expression).is_none(),
    }
}
