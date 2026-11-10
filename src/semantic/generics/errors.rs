use crate::lexer::SourceSpan;

use crate::semantic::errors::{SemanticError, SemanticErrorKind};

pub(in crate::semantic) fn arity_error(
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
