use crate::ast::{Effect, MetaAttribute};
use crate::lexer::TokenKind;
use crate::parser::{ParseError, ParseErrorCode, ParseErrorKind, Parser};

impl Parser {
    pub(super) fn parse_effect_metadata(&mut self) -> Result<Vec<MetaAttribute>, ParseError> {
        self.expect_simple(TokenKind::LeftParen, "`(` after `effect`")?;
        let mut attributes = Vec::new();
        loop {
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
            attributes.push(MetaAttribute::Effect(effect));
            if !self.match_simple(TokenKind::Comma) {
                break;
            }
            if self.check_simple(&TokenKind::RightParen) {
                break;
            }
        }
        self.expect_simple(TokenKind::RightParen, "`)` after effect names")?;
        let _ = self.match_simple(TokenKind::Semicolon);
        Ok(attributes)
    }
}
