use crate::lexer::{LexError, LexErrorKind};
use crate::parser::{ParseError, ParseErrorCode, ParseErrorKind};
use crate::semantic::{SemanticError, SemanticErrorKind};

pub fn render_lex_error(source: &str, error: &LexError) -> String {
    let (line, column) = line_column(source, error.span.start);
    let message = match &error.kind {
        LexErrorKind::UnexpectedCharacter(character) => {
            format!("unexpected character `{character}`")
        }
        LexErrorKind::UnterminatedString => "unterminated string literal".to_owned(),
    };

    format!("error[E000{code}] at {line}:{column}: {message}", code = lex_code(error))
}

pub fn render_parse_error(source: &str, error: &ParseError) -> String {
    let (line, column) = line_column(source, error.span.start);
    let message = match &error.kind {
        ParseErrorKind::UnexpectedToken { expected, found } => {
            format!("expected {expected}, found {found:?}")
        }
        ParseErrorKind::UnexpectedEndOfInput { expected } => {
            format!("expected {expected}, found end of input")
        }
        ParseErrorKind::DuplicateName { kind, name } => {
            format!("duplicate {kind} `{name}`")
        }
    };

    format!("error[{}] at {line}:{column}: {message}", parse_code(error.code))
}

pub fn render_semantic_error(source: &str, error: &SemanticError) -> String {
    let (line, column) = line_column(source, error.span.start);
    format!(
        "error[{}] at {line}:{column}: {}",
        semantic_code(&error.kind),
        semantic_message(&error.kind)
    )
}

fn lex_code(error: &LexError) -> u8 {
    match error.kind {
        LexErrorKind::UnexpectedCharacter(_) => 1,
        LexErrorKind::UnterminatedString => 2,
    }
}

fn parse_code(code: ParseErrorCode) -> &'static str {
    match code {
        ParseErrorCode::UnexpectedToken => "E0003",
        ParseErrorCode::UnexpectedEndOfInput => "E0004",
        ParseErrorCode::DuplicateName => "E0005",
    }
}

fn semantic_code(kind: &SemanticErrorKind) -> &'static str {
    match kind {
        SemanticErrorKind::DuplicateBinding { .. } => "E1001",
        SemanticErrorKind::ShadowedBinding { .. } => "E1002",
        SemanticErrorKind::UndeclaredIdentifier { .. } => "E1003",
        SemanticErrorKind::InvalidBorrowTarget { .. } => "E1004",
        SemanticErrorKind::UseAfterMove { .. } => "E1005",
        SemanticErrorKind::UseAfterDrop { .. } => "E1006",
        SemanticErrorKind::DoubleDrop { .. } => "E1007",
        SemanticErrorKind::DropBorrow { .. } => "E1008",
        SemanticErrorKind::DropFrozen { .. } => "E1009",
        SemanticErrorKind::InvalidDatArgument { .. } => "E1010",
        SemanticErrorKind::MoveFrozen { .. } => "E1011",
        SemanticErrorKind::UnknownParameter { .. } => "E1012",
        SemanticErrorKind::DuplicateArgument { .. } => "E1013",
        SemanticErrorKind::MixedArgumentModes { .. } => "E1014",
        SemanticErrorKind::WrongArgumentCount { .. } => "E1015",
        SemanticErrorKind::InvalidArgumentRole { .. } => "E1016",
        SemanticErrorKind::InvalidIntrinsicArgument { .. } => "E1021",
        SemanticErrorKind::AmbiguousPositionalCall { .. } => "E1017",
        SemanticErrorKind::BorrowedReturn { .. } => "E1018",
        SemanticErrorKind::InvalidOwnerInitializer { .. } => "E1019",
        SemanticErrorKind::LoopControlOutsideLoop { .. } => "E1020",
        SemanticErrorKind::ReservedIntrinsicName { .. } => "E1022",
        SemanticErrorKind::UnknownType { .. } => "E1023",
        SemanticErrorKind::DuplicateVerbName { .. } => "E1024",
        SemanticErrorKind::DuplicateStructName { .. } => "E1029",
        SemanticErrorKind::DuplicateEnumName { .. } => "E1039",
        SemanticErrorKind::DuplicateStructField { .. } => "E1030",
        SemanticErrorKind::UnknownStructField { .. } => "E1031",
        SemanticErrorKind::MissingStructField { .. } => "E1032",
        SemanticErrorKind::StructFieldTypeMismatch { .. } => "E1033",
        SemanticErrorKind::TypeMismatch { .. } => "E1025",
        SemanticErrorKind::ReturnTypeMismatch { .. } => "E1026",
        SemanticErrorKind::BindingTypeMismatch { .. } => "E1027",
        SemanticErrorKind::MissingReturnValue => "E1028",
        SemanticErrorKind::InvalidFieldAssignmentTarget { .. } => "E1034",
        SemanticErrorKind::FieldBorrowConflict { .. } => "E1035",
        SemanticErrorKind::UnknownMethod { .. } => "E1036",
        SemanticErrorKind::InvalidReceiver { .. } => "E1037",
        SemanticErrorKind::ReceiverTypeMismatch { .. } => "E1038",
        SemanticErrorKind::UnknownEnumVariant { .. } => "E1040",
        SemanticErrorKind::EnumVariantArgumentCount { .. } => "E1041",
        SemanticErrorKind::EnumVariantArgumentName { .. } => "E1042",
        SemanticErrorKind::EnumVariantArgumentTypeMismatch { .. } => "E1043",
        SemanticErrorKind::RecursiveType { .. } => "E1044",
        SemanticErrorKind::NonExhaustiveMatch { .. } => "E1045",
        SemanticErrorKind::UnreachablePattern { .. } => "E1046",
        SemanticErrorKind::DuplicatePattern { .. } => "E1047",
        SemanticErrorKind::PatternTypeMismatch { .. } => "E1048",
        SemanticErrorKind::PatternBindingTypeMismatch { .. } => "E1049",
        SemanticErrorKind::InvalidCaseRole { .. } => "E1050",
        SemanticErrorKind::InvalidMutation { .. } => "E1051",
    }
}

fn semantic_message(kind: &SemanticErrorKind) -> String {
    if let Some(message) = extended_semantic_message(kind) {
        return message;
    }
    match kind {
        SemanticErrorKind::DuplicateBinding { name } => format!("duplicate binding `{name}`"),
        SemanticErrorKind::ShadowedBinding { name } => format!("shadowed binding `{name}`"),
        SemanticErrorKind::UndeclaredIdentifier { name } => {
            format!("undeclared identifier `{name}`")
        }
        SemanticErrorKind::InvalidBorrowTarget { name } => {
            format!("invalid borrow target `{name}`")
        }
        SemanticErrorKind::UseAfterMove { name } => format!("use of moved binding `{name}`"),
        SemanticErrorKind::UseAfterDrop { name } => format!("use of dropped binding `{name}`"),
        SemanticErrorKind::DoubleDrop { name } => format!("binding `{name}` was dropped twice"),
        SemanticErrorKind::DropBorrow { name } => format!("cannot drop borrow `{name}` directly"),
        SemanticErrorKind::DropFrozen { name, .. } => {
            format!("cannot drop frozen binding `{name}`")
        }
        SemanticErrorKind::InvalidDatArgument { name } => format!("invalid dat argument `{name}`"),
        SemanticErrorKind::MoveFrozen { name, .. } => {
            format!("cannot move frozen binding `{name}`")
        }
        SemanticErrorKind::UnknownParameter { callee, name } => {
            format!("unknown parameter `{name}` in call to `{callee}`")
        }
        SemanticErrorKind::DuplicateArgument { name } => format!("duplicate argument `{name}`"),
        SemanticErrorKind::MixedArgumentModes { callee } => {
            format!("cannot mix named and positional arguments in `{callee}`")
        }
        SemanticErrorKind::WrongArgumentCount { callee } => {
            format!("wrong argument count in `{callee}`")
        }
        SemanticErrorKind::InvalidArgumentRole { callee, parameter } => {
            format!("argument does not satisfy role of `{parameter}` in `{callee}`")
        }
        SemanticErrorKind::InvalidIntrinsicArgument { callee, parameter } => {
            format!("invalid `{parameter}` argument in intrinsic `{callee}`")
        }
        SemanticErrorKind::AmbiguousPositionalCall { callee } => {
            format!("positional call to `{callee}` is ambiguous")
        }
        SemanticErrorKind::BorrowedReturn { name } => {
            format!("borrow `{name}` cannot escape its scope")
        }
        SemanticErrorKind::InvalidOwnerInitializer { name } => {
            format!("invalid owner initializer `{name}`")
        }
        SemanticErrorKind::LoopControlOutsideLoop { keyword } => {
            format!("`{keyword}` is only valid inside a loop")
        }
        SemanticErrorKind::ReservedIntrinsicName { name } => {
            format!("`{name}` is reserved for a built-in intrinsic")
        }
        _ => unreachable!("extended semantic message was not rendered"),
    }
}

fn extended_semantic_message(kind: &SemanticErrorKind) -> Option<String> {
    enum_semantic_message(kind)
        .or_else(|| struct_semantic_message(kind))
        .or_else(|| type_semantic_message(kind))
}

fn enum_semantic_message(kind: &SemanticErrorKind) -> Option<String> {
    let message = match kind {
        SemanticErrorKind::DuplicateEnumName { name } => {
            format!("duplicate enum declaration `{name}`")
        }
        SemanticErrorKind::UnknownEnumVariant { enum_name, variant } => {
            format!("unknown variant `{variant}` for enum `{enum_name}`")
        }
        SemanticErrorKind::EnumVariantArgumentCount { enum_name, variant, expected, found } => {
            format!(
                "wrong argument count for `{enum_name}.{variant}`: expected {expected}, found {found}"
            )
        }
        SemanticErrorKind::EnumVariantArgumentName { enum_name, variant, name } => {
            format!("unknown field `{name}` for variant `{enum_name}.{variant}`")
        }
        SemanticErrorKind::EnumVariantArgumentTypeMismatch {
            variant,
            parameter,
            expected,
            found,
        } => format!(
            "type mismatch for variant `{variant}` field `{parameter}`: expected `{expected}`, found `{found}`"
        ),
        SemanticErrorKind::RecursiveType { name } => {
            format!("recursive type `{name}` requires indirection")
        }
        SemanticErrorKind::NonExhaustiveMatch { subject, missing } => {
            format!("non-exhaustive match on `{subject}`; missing: {}", missing.join(", "))
        }
        SemanticErrorKind::UnreachablePattern { pattern } => {
            format!("unreachable pattern `{pattern}`")
        }
        SemanticErrorKind::DuplicatePattern { pattern } => {
            format!("duplicate pattern `{pattern}`")
        }
        SemanticErrorKind::PatternTypeMismatch { expected, found } => {
            format!("pattern type mismatch: expected `{expected}`, found `{found}`")
        }
        SemanticErrorKind::PatternBindingTypeMismatch { binding, expected, found } => {
            format!("pattern binding `{binding}` has type `{found}`, expected `{expected}`")
        }
        SemanticErrorKind::InvalidCaseRole { mode, subject } => {
            format!("cannot use `{mode}` case deconstruction on `{subject}`")
        }
        SemanticErrorKind::InvalidMutation { name } => {
            format!("cannot mutate read-only binding `{name}`")
        }
        _ => return None,
    };
    Some(message)
}

fn struct_semantic_message(kind: &SemanticErrorKind) -> Option<String> {
    let message = match kind {
        SemanticErrorKind::DuplicateStructName { name } => {
            format!("duplicate struct declaration `{name}`")
        }
        SemanticErrorKind::DuplicateStructField { struct_name, field } => {
            format!("duplicate field `{field}` in struct `{struct_name}`")
        }
        SemanticErrorKind::UnknownStructField { struct_name, field } => {
            format!("unknown field `{field}` on struct `{struct_name}`")
        }
        SemanticErrorKind::MissingStructField { struct_name, field } => {
            format!("missing field `{field}` in `{struct_name}` initializer")
        }
        SemanticErrorKind::StructFieldTypeMismatch { struct_name, field, expected, found } => {
            format!(
                "type mismatch for field `{field}` in `{struct_name}`: expected `{expected}`, found `{found}`"
            )
        }
        SemanticErrorKind::InvalidFieldAssignmentTarget { field } => {
            format!("field `{field}` can only be assigned through an erg owner")
        }
        SemanticErrorKind::FieldBorrowConflict { owner, field, .. } => {
            format!("cannot mutate or move field `{field}` because owner `{owner}` is frozen")
        }
        _ => return None,
    };
    Some(message)
}

fn type_semantic_message(kind: &SemanticErrorKind) -> Option<String> {
    let message = match kind {
        SemanticErrorKind::UnknownType { name } => format!("unknown type `{name}`"),
        SemanticErrorKind::DuplicateVerbName { name } => {
            format!("duplicate verb declaration `{name}`")
        }
        SemanticErrorKind::TypeMismatch { callee, parameter, expected, found } => format!(
            "type mismatch for `{parameter}` in `{callee}`: expected `{expected}`, found `{found}`"
        ),
        SemanticErrorKind::ReturnTypeMismatch { expected, found } => {
            format!("return type mismatch: expected `{expected}`, found `{found}`")
        }
        SemanticErrorKind::BindingTypeMismatch { binding, expected, found } => {
            format!("type mismatch for `{binding}`: expected `{expected}`, found `{found}`")
        }
        SemanticErrorKind::UnknownMethod { method } => format!("unknown method `{method}`"),
        SemanticErrorKind::InvalidReceiver { method } => {
            format!("invalid receiver for method `{method}`")
        }
        SemanticErrorKind::ReceiverTypeMismatch { method, expected, found } => {
            format!("receiver type mismatch for `{method}`: expected `{expected}`, found `{found}`")
        }
        SemanticErrorKind::MissingReturnValue => {
            "verb must return a value on every path".to_owned()
        }
        _ => return None,
    };
    Some(message)
}

fn line_column(source: &str, byte_offset: usize) -> (usize, usize) {
    let offset = byte_offset.min(source.len());
    let prefix = &source[..offset];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix.rsplit('\n').next().map_or(1, |line| line.chars().count() + 1);
    (line, column)
}
