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
/// Analyzes a parsed program that has no compilation plan. Its types are
/// resolved against the program's own declarations first (ADR-0108).
pub fn analyze(program: &Program) -> Result<SemanticModel, SemanticError> {
    let mut resolved = program.clone();
    crate::modules::DeclarationRegistry::resolve_standalone(&mut resolved);
    analyze_resolved(&resolved)
}

/// Analyzes a program whose types a compilation plan already annotated.
pub(crate) fn analyze_resolved(program: &Program) -> Result<SemanticModel, SemanticError> {
    let normalized = crate::semantic::normalize_generated_locals(program);
    let model = Analyzer::new().analyze(&normalized)?;
    // A `?` that no rule above rejected and that is not a statement of its
    // own would not return early when lowered (bug #130).
    if let Some((span, place)) = crate::semantic::misplaced_try(&normalized) {
        return Err(SemanticError {
            kind: super::errors::SemanticErrorKind::MisplacedTry(place),
            span,
        });
    }
    Ok(model)
}
