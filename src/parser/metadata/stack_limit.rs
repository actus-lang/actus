use crate::ast::{MetaAttribute, StackLimitContract};
use crate::lexer::TokenKind;
use crate::parser::state::identifier_text;
use crate::parser::{ParseError, ParseErrorCode, ParseErrorKind, Parser};

impl Parser {
    pub(super) fn parse_stack_limit_metadata(&mut self) -> Result<Vec<MetaAttribute>, ParseError> {
        self.expect_simple(TokenKind::LeftParen, "`(` after `stack_limit`")?;
        let field = self.take_identifier("stack_limit parameter name")?;
        if identifier_text(&field.kind) != "bytes" {
            return Err(
                self.invalid_stack_limit("the only supported parameter is `bytes`", field.span)
            );
        }
        self.expect_simple(TokenKind::Colon, "`:` after `bytes`")?;
        let value = self.advance_required("stack_limit byte limit")?;
        let bytes = match value.kind {
            TokenKind::Integer { value, suffix: Some(suffix) } if suffix == "u32" => {
                value.parse::<u32>().ok().filter(|value| *value > 0)
            }
            _ => None,
        };
        let Some(bytes) = bytes else {
            return Err(self.invalid_stack_limit(
                "`bytes` must be a positive `u32` integer literal",
                value.span,
            ));
        };
        if self.match_simple(TokenKind::Comma) && !self.check_simple(&TokenKind::RightParen) {
            return Err(self.invalid_stack_limit(
                "`bytes` is the only supported parameter",
                self.metadata_span(),
            ));
        }
        self.expect_simple(TokenKind::RightParen, "`)` after stack limit")?;
        let _ = self.match_simple(TokenKind::Semicolon);
        Ok(vec![MetaAttribute::StackLimit(StackLimitContract { bytes })])
    }

    fn invalid_stack_limit(&self, reason: &str, span: crate::lexer::SourceSpan) -> ParseError {
        ParseError {
            code: ParseErrorCode::InvalidStackLimit,
            kind: ParseErrorKind::InvalidStackLimit { reason: reason.to_owned() },
            span,
        }
    }
}
