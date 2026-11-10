mod kinds;

pub use kinds::SemanticErrorKind;

use crate::lexer::SourceSpan;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticError {
    pub kind: SemanticErrorKind,
    pub span: SourceSpan,
}
