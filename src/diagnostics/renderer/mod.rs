mod codes;

use crate::lexer::{LexError, LexErrorKind};
use crate::parser::{ParseError, ParseErrorKind};
use crate::semantic::SemanticError;

use super::{Diagnostic, DiagnosticSeverity};
use codes::{lex_code, parse_code, semantic_code, semantic_message};

/// Converts a lexical failure into the stable diagnostic model.
pub fn lex_diagnostic(error: &LexError) -> Diagnostic {
    Diagnostic::error(format!("E000{}", lex_code(error)), error.span, lex_message(error))
}

/// Converts a parser failure into the stable diagnostic model.
pub fn parse_diagnostic(error: &ParseError) -> Diagnostic {
    Diagnostic::error(parse_code(error.code), error.span, parse_message(error))
}

/// Converts a semantic failure into the stable diagnostic model.
pub fn semantic_diagnostic(error: &SemanticError) -> Diagnostic {
    Diagnostic::error(semantic_code(&error.kind), error.span, semantic_message(&error.kind))
}

pub fn render_lex_error(source: &str, error: &LexError) -> String {
    render_diagnostic(source, &lex_diagnostic(error))
}

pub fn render_parse_error(source: &str, error: &ParseError) -> String {
    render_diagnostic(source, &parse_diagnostic(error))
}

pub fn render_semantic_error(source: &str, error: &SemanticError) -> String {
    render_diagnostic(source, &semantic_diagnostic(error))
}

/// Renders one diagnostic for the terminal while preserving its stable code
/// and source location.
pub fn render_diagnostic(source: &str, diagnostic: &Diagnostic) -> String {
    let (line, column) = line_column(source, diagnostic.span().start);
    let severity = match diagnostic.severity() {
        DiagnosticSeverity::Error => "error",
        DiagnosticSeverity::Warning => "warning",
    };
    format!("{severity}[{}] at {line}:{column}: {}", diagnostic.code(), diagnostic.message())
}

fn lex_message(error: &LexError) -> String {
    match &error.kind {
        LexErrorKind::UnexpectedCharacter(character) => {
            format!("unexpected character `{character}`")
        }
        LexErrorKind::UnterminatedString => "unterminated string literal".to_owned(),
        LexErrorKind::UnterminatedDocString => "unterminated docstring literal".to_owned(),
        LexErrorKind::InvalidIntegerType(text) => {
            format!("invalid integer type `{text}`; expected `u1..u128` or `i1..i128`")
        }
        LexErrorKind::InvalidHexLiteral(text) => format!("invalid hexadecimal literal `{text}`"),
    }
}

fn parse_message(error: &ParseError) -> String {
    match &error.kind {
        ParseErrorKind::UnexpectedToken { expected, found } => {
            format!("expected {expected}, found {found:?}")
        }
        ParseErrorKind::UnexpectedEndOfInput { expected } => {
            format!("expected {expected}, found end of input")
        }
        ParseErrorKind::DuplicateName { kind, name } => {
            format!("duplicate {kind} `{name}`")
        }
    }
}

fn line_column(source: &str, byte_offset: usize) -> (usize, usize) {
    let offset = byte_offset.min(source.len());
    let prefix = &source[..offset];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix.rsplit('\n').next().map_or(1, |line| line.chars().count() + 1);
    (line, column)
}
