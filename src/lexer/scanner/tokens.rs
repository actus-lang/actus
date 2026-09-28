use super::identifiers::is_identifier_start;
use super::{LexError, LexErrorKind, Scanner};
use crate::lexer::{SourceSpan, Token, TokenKind};

impl<'source> Scanner<'source> {
    pub(super) fn scan_token(&mut self) {
        let start = self.cursor;
        let character = self.advance().expect("scan_token called at end of input");
        match character {
            '{' => self.push_simple(TokenKind::LeftBrace, start),
            '}' => self.push_simple(TokenKind::RightBrace, start),
            '(' => self.push_simple(TokenKind::LeftParen, start),
            ')' => self.push_simple(TokenKind::RightParen, start),
            '[' => self.push_simple(TokenKind::LeftBracket, start),
            ']' => self.push_simple(TokenKind::RightBracket, start),
            ':' => self.push_simple(TokenKind::Colon, start),
            ',' => self.push_simple(TokenKind::Comma, start),
            ';' => self.push_simple(TokenKind::Semicolon, start),
            '<' if self.match_character('=') => self.push_simple(TokenKind::LessEquals, start),
            '<' => self.push_simple(TokenKind::LessThan, start),
            '=' if self.match_character('>') => self.push_simple(TokenKind::FatArrow, start),
            '=' if self.match_character('=') => self.push_simple(TokenKind::DoubleEquals, start),
            '=' => self.push_simple(TokenKind::Equals, start),
            '>' if self.match_character('=') => self.push_simple(TokenKind::GreaterEquals, start),
            '>' => self.push_simple(TokenKind::GreaterThan, start),
            '!' if self.match_character('=') => self.push_simple(TokenKind::BangEquals, start),
            '!' => self.push_simple(TokenKind::Bang, start),
            '+' => self.push_simple(TokenKind::Plus, start),
            '-' if self.match_character('>') => self.push_simple(TokenKind::Arrow, start),
            '-' => self.push_simple(TokenKind::Minus, start),
            '*' => self.push_simple(TokenKind::Star, start),
            '/' => self.push_simple(TokenKind::Slash, start),
            '.' => self.push_simple(TokenKind::Dot, start),
            '?' => self.push_simple(TokenKind::Question, start),
            '0' if matches!(self.peek(), Some('x' | 'X')) => self.scan_hex_integer(start),
            '"' if self.peek() == Some('"') && self.peek_next() == Some('"') => {
                self.advance();
                self.advance();
                self.scan_doc_string(start);
            }
            '"' => self.scan_string(start),
            character if is_identifier_start(character) => self.scan_identifier(start),
            character if character.is_ascii_digit() => self.scan_integer(start),
            character => self.errors.push(LexError::new(
                LexErrorKind::UnexpectedCharacter(character),
                SourceSpan::new(start, self.cursor),
            )),
        }
    }

    pub(super) fn skip_whitespace_and_comments(&mut self) {
        loop {
            while self.peek().is_some_and(char::is_whitespace) {
                self.advance();
            }
            if self.peek() != Some('/') || self.peek_next() != Some('/') {
                return;
            }
            while self.peek().is_some_and(|character| character != '\n') {
                self.advance();
            }
        }
    }

    fn push_simple(&mut self, kind: TokenKind, start: usize) {
        self.tokens.push(Token::new(kind, SourceSpan::new(start, self.cursor)));
    }
}
