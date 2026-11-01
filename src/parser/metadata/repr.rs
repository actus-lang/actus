use super::super::{ParseError, ParseErrorCode, ParseErrorKind, Parser};
use crate::ast::{MetaAttribute, Repr};
use crate::lexer::TokenKind;

impl Parser {
    pub(super) fn parse_repr_metadata(&mut self) -> Result<Vec<MetaAttribute>, ParseError> {
        self.expect_simple(TokenKind::LeftParen, "`(` after `repr`")?;
        let mut c = false;
        let mut packed = false;
        let mut transparent = false;
        loop {
            let token = self.take_identifier("representation name")?;
            let name = super::super::identifier_text(&token.kind);
            match name.as_str() {
                "C" if !c => c = true,
                "packed" if !packed => packed = true,
                "transparent" if !transparent => transparent = true,
                "C" | "packed" | "transparent" => {
                    return Err(ParseError {
                        code: ParseErrorCode::DuplicateMetadata,
                        kind: ParseErrorKind::DuplicateMetadata { name: "repr".to_owned() },
                        span: token.span,
                    });
                }
                _ => {
                    return Err(ParseError {
                        code: ParseErrorCode::UnsupportedRepresentation,
                        kind: ParseErrorKind::UnsupportedRepresentation { name },
                        span: token.span,
                    });
                }
            }
            if !self.match_simple(TokenKind::Comma) {
                break;
            }
        }
        self.expect_simple(TokenKind::RightParen, "`)` after representation names")?;
        let _ = self.match_simple(TokenKind::Semicolon);
        Ok(vec![MetaAttribute::Repr(Repr::from_flags(c, packed, transparent))])
    }
}
