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
    DropFrozen { name: String, borrow_ids: Vec<usize> },
    InvalidDatArgument { name: String },
    MoveFrozen { name: String, borrow_ids: Vec<usize> },
    UnknownParameter { callee: String, name: String },
    DuplicateArgument { name: String },
    MixedArgumentModes { callee: String },
    WrongArgumentCount { callee: String },
    InvalidArgumentRole { callee: String, parameter: String },
    InvalidIntrinsicArgument { callee: String, parameter: String },
    AmbiguousPositionalCall { callee: String },
    BorrowedReturn { name: String },
    InvalidOwnerInitializer { name: String },
    LoopControlOutsideLoop { keyword: String },
    ReservedIntrinsicName { name: String },
    UnknownType { name: String },
    DuplicateVerbName { name: String },
    DuplicateStructName { name: String },
    DuplicateStructField { struct_name: String, field: String },
    UnknownStructField { struct_name: String, field: String },
    MissingStructField { struct_name: String, field: String },
    StructFieldTypeMismatch { struct_name: String, field: String, expected: String, found: String },
    TypeMismatch { callee: String, parameter: String, expected: String, found: String },
    ReturnTypeMismatch { expected: String, found: String },
    BindingTypeMismatch { binding: String, expected: String, found: String },
    InvalidFieldAssignmentTarget { field: String },
    FieldBorrowConflict { owner: String, field: String, borrow_ids: Vec<usize> },
    UnknownMethod { method: String },
    InvalidReceiver { method: String },
    ReceiverTypeMismatch { method: String, expected: String, found: String },
    MissingReturnValue,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticError {
    pub kind: SemanticErrorKind,
    pub span: SourceSpan,
}
