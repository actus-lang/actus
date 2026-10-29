use super::super::{ParseError, ParseErrorCode, ParseErrorKind, Parser};
use crate::ast::{Effect, MetaAttribute};
use crate::lexer::TokenKind;

impl Parser {
    pub(super) fn parse_effect_metadata(&mut self) -> Result<Vec<MetaAttribute>, ParseError> {
        self.expect_simple(TokenKind::LeftParen, "`(` after `effect`")?;
        let effect_token = self.advance_required("effect name string")?;
        let name = match effect_token.kind {
            TokenKind::StringLiteral(name) => name,
            found => {
                return Err(super::syntax::unexpected_metadata_token(
                    "an effect name string",
                    found,
                    effect_token.span,
                ));
            }
        };
        let effect = Effect::parse(&name).ok_or(ParseError {
            code: ParseErrorCode::UnsupportedEffect,
            kind: ParseErrorKind::UnsupportedEffect { name },
            span: effect_token.span,
        })?;
        self.expect_simple(TokenKind::RightParen, "`)` after effect name")?;
        let _ = self.match_simple(TokenKind::Semicolon);
        Ok(vec![MetaAttribute::Effect(effect)])
    }
}
