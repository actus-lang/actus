use super::{LexError, LexErrorKind, Scanner};
use crate::lexer::{SourceSpan, Token, TokenKind};

impl<'source> Scanner<'source> {
    pub(super) fn scan_hex_integer(&mut self, start: usize) {
        self.advance();
        let digits_start = self.cursor;
        while self.peek().is_some_and(|character| character.is_ascii_hexdigit()) {
            self.advance();
        }
        let has_digits = self.cursor > digits_start;
        let invalid_suffix = self.peek().is_some_and(is_identifier_continue);
        if invalid_suffix {
            while self.peek().is_some_and(is_identifier_continue) {
                self.advance();
            }
        }
        let text = self.source[start..self.cursor].to_owned();
        if !has_digits || invalid_suffix {
            self.errors.push(LexError::new(
                LexErrorKind::InvalidHexLiteral(text.clone()),
                SourceSpan::new(start, self.cursor),
            ));
        }
        self.tokens.push(Token::new(TokenKind::Integer(text), SourceSpan::new(start, self.cursor)));
    }

    pub(super) fn scan_integer(&mut self, start: usize) {
        while self.peek().is_some_and(|character| character.is_ascii_digit()) {
            self.advance();
        }
        if self.peek() == Some('.')
            && self.peek_next().is_some_and(|character| character.is_ascii_digit())
        {
            self.advance();
            while self.peek().is_some_and(|character| character.is_ascii_digit()) {
                self.advance();
            }
            let text = &self.source[start..self.cursor];
            self.tokens.push(Token::new(
                TokenKind::FloatLiteral(text.to_owned()),
                SourceSpan::new(start, self.cursor),
            ));
            return;
        }
        let text = &self.source[start..self.cursor];
        self.tokens.push(Token::new(
            TokenKind::Integer(text.to_owned()),
            SourceSpan::new(start, self.cursor),
        ));
    }

    pub(super) fn scan_string(&mut self, start: usize) {
        let content_start = self.cursor;
        while let Some(character) = self.peek() {
            match character {
                '"' => {
                    let content = self.source[content_start..self.cursor].to_owned();
                    self.advance();
                    self.tokens.push(Token::new(
                        TokenKind::StringLiteral(content),
                        SourceSpan::new(start, self.cursor),
                    ));
                    return;
                }
                '\\' => {
                    self.advance();
                    if !self.is_at_end() {
                        self.advance();
                    }
                }
                '\n' | '\r' => break,
                _ => {
                    self.advance();
                }
            }
        }
        self.errors.push(LexError::new(
            LexErrorKind::UnterminatedString,
            SourceSpan::new(start, self.cursor),
        ));
    }

    pub(super) fn scan_doc_string(&mut self, start: usize) {
        let content_start = self.cursor;
        while !self.is_at_end() {
            if self.peek() == Some('"')
                && self.peek_next() == Some('"')
                && self.source[self.cursor..].chars().nth(2) == Some('"')
            {
                let content = normalize_doc_string(&self.source[content_start..self.cursor]);
                self.advance();
                self.advance();
                self.advance();
                self.tokens.push(Token::new(
                    TokenKind::DocString(content),
                    SourceSpan::new(start, self.cursor),
                ));
                return;
            }
            self.advance();
        }
        self.errors.push(LexError::new(
            LexErrorKind::UnterminatedDocString,
            SourceSpan::new(start, self.cursor),
        ));
    }
}

fn is_identifier_continue(character: char) -> bool {
    character == '_' || character.is_ascii_alphanumeric()
}

fn normalize_doc_string(content: &str) -> String {
    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines: Vec<&str> = normalized.split('\n').collect();
    while lines.first().is_some_and(|line| line.trim().is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    let indent = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.chars().take_while(|character| character.is_whitespace()).count())
        .min()
        .unwrap_or(0);
    lines.iter().map(|line| remove_indent(line, indent)).collect::<Vec<_>>().join("\n")
}

fn remove_indent(line: &str, indent: usize) -> &str {
    let mut offset = 0;
    for character in line.chars().take(indent) {
        offset += character.len_utf8();
    }
    &line[offset..]
}
