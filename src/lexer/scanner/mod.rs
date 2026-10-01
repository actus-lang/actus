mod cursor;
mod identifiers;
mod literals;
mod tokens;

use super::token::{SourceSpan, Token, TokenKind};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LexErrorKind {
    UnexpectedCharacter(char),
    UnterminatedString,
    UnterminatedDocString,
    InvalidIntegerType(String),
    InvalidFloatType(String),
    InvalidHexLiteral(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LexError {
    pub kind: LexErrorKind,
    pub span: SourceSpan,
}

impl LexError {
    const fn new(kind: LexErrorKind, span: SourceSpan) -> Self {
        Self { kind, span }
    }
}

pub struct Scanner<'source> {
    source: &'source str,
    cursor: usize,
    tokens: Vec<Token>,
    errors: Vec<LexError>,
}

impl<'source> Scanner<'source> {
    pub fn new(source: &'source str) -> Self {
        Self { source, cursor: 0, tokens: Vec::new(), errors: Vec::new() }
    }

    pub fn scan(mut self) -> (Vec<Token>, Vec<LexError>) {
        while !self.is_at_end() {
            self.skip_whitespace();
            if self.is_at_end() {
                break;
            }
            self.scan_token();
        }
        let end = self.cursor;
        self.tokens.push(Token::new(TokenKind::Eof, SourceSpan::new(end, end)));
        (self.tokens, self.errors)
    }

    fn skip_whitespace(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.advance();
        }
    }
}
