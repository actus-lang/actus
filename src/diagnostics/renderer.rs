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
        SemanticErrorKind::TypeMismatch { .. } => "E1025",
        SemanticErrorKind::ReturnTypeMismatch { .. } => "E1026",
        SemanticErrorKind::BindingTypeMismatch { .. } => "E1027",
        SemanticErrorKind::MissingReturnValue => "E1028",
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
