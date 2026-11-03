mod bounded;
mod bounded_graph;
mod constants;
mod declarations;
mod expression_conditionals;
mod expression_context;
mod expression_rules;
mod expressions;
mod for_loops;
mod lifecycle;
mod operator_validation;
mod state;
mod statement_control;
mod statements;
mod validation;
mod verb;

pub(crate) use expression_rules::cast_literal_fits;
pub(crate) use expressions::case_branch_type;
pub(crate) use state::{Analyzer, ScopeFrame};
pub(crate) use validation::{
    canonical_type_name, expression_span, is_origin_return_expression, try_type_mismatch,
    type_contains_float, type_name_text_contains_float,
};

pub(crate) fn validate_bounded_recursion(
    program: &Program,
) -> Result<(), super::errors::SemanticError> {
    Analyzer::new().validate_bounded_recursion(program)
}

use super::errors::SemanticError;
use super::model::SemanticModel;
use crate::ast::Program;

pub fn analyze(program: &Program) -> Result<SemanticModel, SemanticError> {
    let normalized = super::normalize_generated_locals(program);
    Analyzer::new().analyze(&normalized)
}
