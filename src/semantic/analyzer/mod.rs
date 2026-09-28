mod declarations;
mod expressions;
mod lifecycle;
mod state;
mod statement_control;
mod statements;
mod validation;
mod verb;

pub(crate) use state::{Analyzer, ScopeFrame};
pub(crate) use validation::{canonical_type_name, is_origin_return_expression, try_type_mismatch};

use super::errors::SemanticError;
use super::model::SemanticModel;
use crate::ast::Program;

pub fn analyze(program: &Program) -> Result<SemanticModel, SemanticError> {
    Analyzer::new().analyze(program)
}
