use super::super::{ParseError, ParseErrorCode, ParseErrorKind, Parser};
use crate::ast::{MetaAttribute, Repr};
use crate::lexer::TokenKind;

impl Parser {
    pub(super) fn parse_repr_metadata(&mut self) -> Result<Vec<MetaAttribute>, ParseError> {
        self.expect_simple(TokenKind::LeftParen, "`(` after `repr`")?;
        let token = self.take_identifier("representation name")?;
        let name = super::super::identifier_text(&token.kind);
        if name != "C" {
            return Err(ParseError {
                code: ParseErrorCode::UnsupportedRepresentation,
                kind: ParseErrorKind::UnsupportedRepresentation { name },
                span: token.span,
            });
        }
        self.expect_simple(TokenKind::RightParen, "`)` after representation name")?;
        let _ = self.match_simple(TokenKind::Semicolon);
        Ok(vec![MetaAttribute::Repr(Repr::C)])
    }
}
