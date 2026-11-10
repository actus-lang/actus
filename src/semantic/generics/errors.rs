use crate::lexer::SourceSpan;

use super::errors::{SemanticError, SemanticErrorKind};

pub(super) fn arity_error(
    name: &str,
    expected: usize,
    found: usize,
    span: SourceSpan,
) -> SemanticError {
    SemanticError {
        kind: SemanticErrorKind::GenericArityMismatch { name: name.to_owned(), expected, found },
        span,
    }
}
