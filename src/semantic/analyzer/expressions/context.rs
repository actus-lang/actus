use crate::ast::Expr;

use super::super::errors::SemanticError;
use super::Analyzer;

impl Analyzer {
    /// Visits the inside of a grouping, which drops its value exactly when the
    /// grouping does.
    pub(crate) fn visit_grouped_expression(
        &mut self,
        expression: &Expr,
        dropped: bool,
    ) -> Result<(), SemanticError> {
        self.drops_next_value = dropped;
        self.visit_expression(expression)
    }

    pub(crate) fn visit_consumed_expression(
        &mut self,
        expression: &Expr,
    ) -> Result<(), SemanticError> {
        let previous = self.expression_value_consumed;
        self.expression_value_consumed = true;
        let result = self.visit_expression(expression);
        self.expression_value_consumed = previous;
        result
    }
}
