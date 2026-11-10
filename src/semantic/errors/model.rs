use super::kinds::SemanticErrorKind;
use crate::lexer::SourceSpan;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticError {
    pub kind: SemanticErrorKind,
    pub span: SourceSpan,
}
