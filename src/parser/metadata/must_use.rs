use crate::ast::MetaAttribute;
use crate::lexer::TokenKind;
use crate::parser::{ParseError, Parser};

impl Parser {
    pub(super) fn parse_must_use_metadata(&mut self) -> Result<Vec<MetaAttribute>, ParseError> {
        self.expect_simple(TokenKind::LeftParen, "`(` after `must_use`")?;
        let reason_token = self.advance_required("must-use reason string")?;
        let reason = match reason_token.kind {
            TokenKind::StringLiteral(reason) if !reason.trim().is_empty() => reason,
            TokenKind::StringLiteral(_) => {
                return Err(super::syntax::unexpected_metadata_token(
                    "a non-empty must-use reason string",
                    reason_token.kind,
                    reason_token.span,
                ));
            }
            found => {
                return Err(super::syntax::unexpected_metadata_token(
                    "a must-use reason string",
                    found,
                    reason_token.span,
                ));
            }
        };
        self.expect_simple(TokenKind::RightParen, "`)` after must-use reason")?;
        let _ = self.match_simple(TokenKind::Semicolon);
        Ok(vec![MetaAttribute::MustUse(reason)])
    }
}
