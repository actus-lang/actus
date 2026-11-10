use crate::ast::{MetaAttribute, Repr};
use crate::lexer::TokenKind;
use crate::parser::{ParseError, ParseErrorCode, ParseErrorKind, Parser};

impl Parser {
    pub(super) fn parse_repr_metadata(&mut self) -> Result<Vec<MetaAttribute>, ParseError> {
        self.expect_simple(TokenKind::LeftParen, "`(` after `repr`")?;
        let mut c = false;
        let mut packed = false;
        let mut transparent = false;
        let mut alignment = None;
        loop {
            let token = self.take_identifier("representation name")?;
            let name = crate::parser::state::identifier_text(&token.kind);
            match name.as_str() {
                "C" if !c => c = true,
                "packed" if !packed => packed = true,
                "transparent" if !transparent => transparent = true,
                "align" if alignment.is_none() => {
                    self.expect_simple(TokenKind::LeftParen, "`(` after `align`")?;
                    let value = self.advance_required("alignment value")?;
                    alignment = Some(parse_alignment(value)?);
                    self.expect_simple(TokenKind::RightParen, "`)` after alignment value")?;
                }
                "C" | "packed" | "transparent" | "align" => {
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
        Ok(vec![MetaAttribute::Repr(
            Repr::from_flags(c, packed, transparent).with_optional_alignment(alignment),
        )])
    }
}

fn parse_alignment(token: crate::lexer::Token) -> Result<u32, ParseError> {
    let span = token.span;
    match token.kind {
        TokenKind::Integer { value, suffix: Some(suffix) } if suffix == "u32" => {
            value.parse::<u32>().ok().filter(|value| *value > 0).ok_or(ParseError {
                code: ParseErrorCode::UnsupportedRepresentation,
                kind: ParseErrorKind::UnsupportedRepresentation { name: "align".to_owned() },
                span,
            })
        }
        _ => Err(ParseError {
            code: ParseErrorCode::UnsupportedRepresentation,
            kind: ParseErrorKind::UnsupportedRepresentation { name: "align".to_owned() },
            span,
        }),
    }
}
