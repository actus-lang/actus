use super::state::Analyzer;
use crate::ast::Program;

pub(crate) fn validate_bounded_recursion(
    program: &Program,
) -> Result<(), super::errors::SemanticError> {
    Analyzer::new().validate_bounded_recursion(program)
}

pub(crate) fn validate_stack_limit_recursion(
    program: &Program,
) -> Result<(), super::errors::SemanticError> {
    Analyzer::new().validate_stack_limit_recursion(program)
}

pub(crate) fn validate_stack_limit_native_claims(
    program: &Program,
) -> Result<(), super::errors::SemanticError> {
    Analyzer::new().validate_stack_limit_native_claims(program)
}

pub(crate) fn validate_stack_limit_dynamic_dispatch(
    program: &Program,
) -> Result<(), super::errors::SemanticError> {
    Analyzer::new().validate_stack_limit_dynamic_dispatch(program)
}

use super::errors::SemanticError;
use super::model::SemanticModel;
pub fn analyze(program: &Program) -> Result<SemanticModel, SemanticError> {
    let normalized = super::normalize_generated_locals(program);
    Analyzer::new().analyze(&normalized)
}
