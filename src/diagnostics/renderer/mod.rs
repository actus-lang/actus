mod codes;

use crate::lexer::{LexError, LexErrorKind};
use crate::parser::{ParseError, ParseErrorKind};
use crate::semantic::SemanticError;

use codes::{lex_code, parse_code, semantic_code, semantic_message};

pub fn render_lex_error(source: &str, error: &LexError) -> String {
    let (line, column) = line_column(source, error.span.start);
    let message = match &error.kind {
        LexErrorKind::UnexpectedCharacter(character) => {
            format!("unexpected character `{character}`")
        }
        LexErrorKind::UnterminatedString => "unterminated string literal".to_owned(),
        LexErrorKind::UnterminatedDocString => "unterminated docstring literal".to_owned(),
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

fn line_column(source: &str, byte_offset: usize) -> (usize, usize) {
    let offset = byte_offset.min(source.len());
    let prefix = &source[..offset];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix.rsplit('\n').next().map_or(1, |line| line.chars().count() + 1);
    (line, column)
}
