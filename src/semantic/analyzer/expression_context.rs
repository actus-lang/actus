use crate::ast::Expr;

use super::super::errors::SemanticError;
use super::Analyzer;

impl Analyzer {
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
