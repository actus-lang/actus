use crate::ast::{LimitlessScope, MetaAttribute};
use crate::lexer::TokenKind;
use crate::parser::{ParseError, ParseErrorCode, ParseErrorKind, Parser};

impl Parser {
    pub(super) fn parse_limitless_metadata(&mut self) -> Result<Vec<MetaAttribute>, ParseError> {
        self.expect_simple(TokenKind::LeftParen, "`(` after `limitless`")?;
        let scope_token = self.advance_required("limitless scope string")?;
        let scope = match scope_token.kind {
            TokenKind::StringLiteral(scope) => match scope.as_str() {
                "verb" => LimitlessScope::Verb,
                "file" => LimitlessScope::File,
                _ => {
                    return Err(ParseError {
                        code: ParseErrorCode::UnsupportedLimitlessScope,
                        kind: ParseErrorKind::UnsupportedLimitlessScope { name: scope },
                        span: scope_token.span,
                    });
                }
            },
            found => {
                return Err(super::syntax::unexpected_metadata_token(
                    "`verb` or `file` scope string",
                    found,
                    scope_token.span,
                ));
            }
        };
        self.expect_simple(TokenKind::RightParen, "`)` after limitless scope")?;
        let _ = self.match_simple(TokenKind::Semicolon);
        Ok(vec![MetaAttribute::Limitless(scope)])
    }
}
