use crate::lexer::{LexError, LexErrorKind};
use crate::parser::{ParseError, ParseErrorCode, ParseErrorKind};

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

fn line_column(source: &str, byte_offset: usize) -> (usize, usize) {
    let offset = byte_offset.min(source.len());
    let prefix = &source[..offset];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix.rsplit('\n').next().map_or(1, |line| line.chars().count() + 1);
    (line, column)
}
