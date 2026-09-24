use crate::lexer::{SourceSpan, Token, TokenKind};

use super::{ParseError, ParseErrorCode, ParseErrorKind, Parser};

impl Parser {
    pub(super) fn take_identifier(&mut self, expected: &str) -> Result<Token, ParseError> {
        let token = self.advance_required(expected)?;
        if matches!(token.kind, TokenKind::Identifier(_)) {
            Ok(token)
        } else {
            Err(ParseError {
                code: ParseErrorCode::UnexpectedToken,
                kind: ParseErrorKind::UnexpectedToken {
                    expected: expected.to_owned(),
                    found: token.kind,
                },
                span: token.span,
            })
        }
    }

    pub(super) fn expect_keyword(
        &mut self,
        expected: TokenKind,
        label: &str,
    ) -> Result<Token, ParseError> {
        self.expect_simple(expected, label)
    }

    pub(super) fn expect_simple(
        &mut self,
        expected: TokenKind,
        label: &str,
    ) -> Result<Token, ParseError> {
        if self.check_simple(&expected) {
            Ok(self.advance_required(label)?)
        } else {
            Err(self.error_at_current(label))
        }
    }

    pub(super) fn match_simple(&mut self, expected: TokenKind) -> bool {
        if self.check_simple(&expected) {
            self.cursor += 1;
            true
        } else {
            false
        }
    }

    pub(super) fn check_simple(&self, expected: &TokenKind) -> bool {
        self.peek().is_some_and(|token| token.kind == *expected)
    }

    pub(super) fn check_role(&self, expected: &TokenKind) -> bool {
        self.check_simple(expected)
    }

    pub(super) fn check_identifier(&self) -> bool {
        self.peek().is_some_and(|token| matches!(token.kind, TokenKind::Identifier(_)))
    }

    pub(super) fn peek_next_is(&self, expected: &TokenKind) -> bool {
        self.tokens.get(self.cursor + 1).is_some_and(|token| token.kind == *expected)
    }

    pub(super) fn peek_next_is_identifier(&self) -> bool {
        self.tokens
            .get(self.cursor + 1)
            .is_some_and(|token| matches!(token.kind, TokenKind::Identifier(_)))
    }

    pub(super) fn advance_required(&mut self, expected: &str) -> Result<Token, ParseError> {
        if let Some(token) = self.tokens.get(self.cursor).cloned() {
            self.cursor += 1;
            Ok(token)
        } else {
            Err(ParseError {
                code: ParseErrorCode::UnexpectedEndOfInput,
                kind: ParseErrorKind::UnexpectedEndOfInput { expected: expected.to_owned() },
                span: SourceSpan::new(0, 0),
            })
        }
    }

    pub(super) fn error_at_current(&self, expected: &str) -> ParseError {
        match self.peek() {
            Some(token) => ParseError {
                code: ParseErrorCode::UnexpectedToken,
                kind: ParseErrorKind::UnexpectedToken {
                    expected: expected.to_owned(),
                    found: token.kind.clone(),
                },
                span: token.span,
            },
            None => ParseError {
                code: ParseErrorCode::UnexpectedEndOfInput,
                kind: ParseErrorKind::UnexpectedEndOfInput { expected: expected.to_owned() },
                span: SourceSpan::new(0, 0),
            },
        }
    }

    pub(super) fn previous(&self) -> &Token {
        &self.tokens[self.cursor - 1]
    }

    pub(super) fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.cursor)
    }

    pub(super) fn at_end(&self) -> bool {
        self.check_simple(&TokenKind::Eof)
    }
}
