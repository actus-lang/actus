use crate::ast::{Expr, IfBranch};

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
        self.visit_block(then_branch)?;
        if let Some(else_branch) = else_branch {
            match else_branch {
                IfBranch::Block(block) => self.visit_block(block)?,
                IfBranch::ElseIf(expression) => self.visit_expression(expression)?,
            }
        }
        Ok(())
    }
}
