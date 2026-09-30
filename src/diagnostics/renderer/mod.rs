mod codes;
mod json;
mod module;

use crate::lexer::{LexError, LexErrorKind};
use crate::parser::{ParseError, ParseErrorKind};
use crate::semantic::SemanticError;

use super::{Diagnostic, DiagnosticPhase, DiagnosticSeverity};
use codes::{lex_code, parse_code, semantic_code, semantic_message};

pub use json::render_json_diagnostics;
pub use module::module_diagnostic;

/// Converts a lexical failure into the stable diagnostic model.
pub fn lex_diagnostic(error: &LexError) -> Diagnostic {
    Diagnostic::error(format!("E000{}", lex_code(error)), error.span, lex_message(error))
        .with_phase(DiagnosticPhase::Lexical)
}

/// Converts a parser failure into the stable diagnostic model.
pub fn parse_diagnostic(error: &ParseError) -> Diagnostic {
    let code = match error.kind {
        ParseErrorKind::UnexpectedToken { found: crate::lexer::TokenKind::Eof, .. }
        | ParseErrorKind::UnexpectedEndOfInput { .. } => "E0004",
        _ => parse_code(error.code),
    };
    let diagnostic = Diagnostic::error(code, error.span, parse_message(error))
        .with_phase(DiagnosticPhase::Parser);
    match &error.kind {
        ParseErrorKind::UnexpectedEndOfInput { expected }
        | ParseErrorKind::UnexpectedToken { expected, found: crate::lexer::TokenKind::Eof } => {
            diagnostic
                .with_explanation("the source ends before the parser can complete this construct")
                .with_suggestion(format!("complete the declaration with {expected}"))
        }
        _ => diagnostic,
    }
}

/// Converts a semantic failure into the stable diagnostic model.
pub fn semantic_diagnostic(error: &SemanticError) -> Diagnostic {
    Diagnostic::error(semantic_code(&error.kind), error.span, semantic_message(&error.kind))
        .with_phase(DiagnosticPhase::Semantic)
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
    let severity = severity_name(diagnostic.severity());
    let mut rendered =
        format!("{severity}[{}] at {line}:{column}: {}", diagnostic.code(), diagnostic.message());
    if let Some(explanation) = diagnostic.explanation() {
        rendered.push_str(&format!("; note: {explanation}"));
    }
    if let Some(suggestion) = diagnostic.suggestion() {
        rendered.push_str(&format!("; help: {suggestion}"));
    }
    rendered
}

/// Renders one diagnostic with optional ANSI severity coloring.
///
/// When `enabled` is false, the result is byte-for-byte identical to the
/// plain renderer. Only the severity label and code receive presentation
/// coloring; the diagnostic content remains unchanged.
pub fn render_colored_diagnostic(source: &str, diagnostic: &Diagnostic, enabled: bool) -> String {
    if !enabled {
        return render_diagnostic(source, diagnostic);
    }
    let (line, column) = line_column(source, diagnostic.span().start);
    let severity = severity_name(diagnostic.severity());
    let color = severity_color(diagnostic.severity());
    format!(
        "{color}{severity}[{}]\x1b[0m at {line}:{column}: {}",
        diagnostic.code(),
        diagnostic.message()
    )
}

fn severity_name(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Error => "error",
        DiagnosticSeverity::Warning => "warning",
    }
}

fn severity_color(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Error => "\x1b[31m",
        DiagnosticSeverity::Warning => "\x1b[33m",
    }
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
        ParseErrorKind::UnknownKeyword { name } => format!("unknown keyword `{name}`"),
        ParseErrorKind::UnknownMetadata { name } => format!("unknown metadata attribute `{name}`"),
        ParseErrorKind::UnsupportedTargetPlatform { name } => {
            format!("unsupported target platform `{name}`")
        }
        ParseErrorKind::ConflictingTargetPlatforms { first, second } => {
            format!("conflicting target platforms `{first}` and `{second}`")
        }
        ParseErrorKind::MetadataTargetNotAllowed => {
            "metadata can only be attached to verbs or external verbs".to_owned()
        }
        ParseErrorKind::MetadataFileScopeNotAllowed => {
            "file-scoped limitless metadata must appear once at the start of the file".to_owned()
        }
        ParseErrorKind::UnsupportedLimitlessScope { name } => {
            format!("unsupported limitless scope `{name}`; expected `verb` or `file`")
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
