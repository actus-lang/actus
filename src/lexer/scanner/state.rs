use crate::lexer::{SourceSpan, Token, TokenKind};

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
    pub(super) const fn new(kind: LexErrorKind, span: SourceSpan) -> Self {
        Self { kind, span }
    }
}

pub struct Scanner<'source> {
    pub(super) source: &'source str,
    pub(super) cursor: usize,
    pub(super) base_offset: usize,
    pub(super) tokens: Vec<Token>,
    pub(super) errors: Vec<LexError>,
}

impl<'source> Scanner<'source> {
    pub fn new(source: &'source str) -> Self {
        Self::new_at(source, 0)
    }

    pub(crate) fn new_at(source: &'source str, base_offset: usize) -> Self {
        Self { source, cursor: 0, base_offset, tokens: Vec::new(), errors: Vec::new() }
    }

    pub(super) fn span(&self, start: usize) -> SourceSpan {
        SourceSpan::new(start + self.base_offset, self.cursor + self.base_offset)
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
        self.tokens.push(Token::new(TokenKind::Eof, self.span(end)));
        (self.tokens, self.errors)
    }

    fn skip_whitespace(&mut self) {
        loop {
            while self.peek().is_some_and(char::is_whitespace) {
                self.advance();
            }
            if self.peek() != Some('#') {
                return;
            }
            while let Some(character) = self.peek() {
                self.advance();
                if character == '\n' {
                    break;
                }
            }
        }
    }
}
