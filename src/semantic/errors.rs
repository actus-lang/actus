use crate::lexer::SourceSpan;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticErrorKind {
    DuplicateBinding { name: String },
    ShadowedBinding { name: String },
    UndeclaredIdentifier { name: String },
    InvalidBorrowTarget { name: String },
    UseAfterMove { name: String },
    UseAfterDrop { name: String },
    DoubleDrop { name: String },
    DropBorrow { name: String },
    DropFrozen { name: String },
    InvalidDatArgument { name: String },
    MoveFrozen { name: String },
    UnknownParameter { callee: String, name: String },
    DuplicateArgument { name: String },
    MixedArgumentModes { callee: String },
    WrongArgumentCount { callee: String },
    InvalidArgumentRole { callee: String, parameter: String },
    AmbiguousPositionalCall { callee: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticError {
    pub kind: SemanticErrorKind,
    pub span: SourceSpan,
}
