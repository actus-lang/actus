use super::super::{ParseError, ParseErrorCode, ParseErrorKind, Parser, identifier_text};
use crate::ast::{BoundedContract, MetaAttribute};
use crate::lexer::TokenKind;

impl Parser {
    pub(super) fn parse_bounded_metadata(&mut self) -> Result<Vec<MetaAttribute>, ParseError> {
        self.expect_simple(TokenKind::LeftParen, "`(` after `bounded`")?;
        let mut contract = BoundedContract {
            max_iterations: None,
            max_call_depth: None,
            max_calls: None,
            max_allocations: None,
            max_allocation_bytes: None,
            max_stack_bytes: None,
            max_io_bytes: None,
        };
        self.parse_bounded_fields(&mut contract)?;
        self.expect_simple(TokenKind::RightParen, "`)` after bounded limit")?;
        let _ = self.match_simple(TokenKind::Semicolon);
        if contract.is_empty() {
            return Err(
                self.invalid_bounded("at least one budget field is required", self.metadata_span())
            );
        }
        Ok(vec![MetaAttribute::Bounded(contract)])
    }

    fn parse_bounded_fields(&mut self, contract: &mut BoundedContract) -> Result<(), ParseError> {
        loop {
            let field = self.take_identifier("bounded parameter name")?;
            self.expect_simple(TokenKind::Colon, "`:` after bounded parameter")?;
            let value = self.advance_required("bounded parameter value")?;
            self.assign_bounded_field(contract, identifier_text(&field.kind).as_str(), value)?;
            if !self.match_simple(TokenKind::Comma) || self.check_simple(&TokenKind::RightParen) {
                break;
            }
        }
        Ok(())
    }

    fn assign_bounded_field(
        &self,
        contract: &mut BoundedContract,
        name: &str,
        token: crate::lexer::Token,
    ) -> Result<(), ParseError> {
        let (value, span) = bounded_integer(token, name)?;
        let narrow_value = || {
            u32::try_from(value)
                .map_err(|_| self.invalid_bounded("u32 budget field is out of range", span))
        };
        let duplicate = match name {
            "max_iterations" => contract.max_iterations.replace(narrow_value()?).is_some(),
            "max_call_depth" => contract.max_call_depth.replace(narrow_value()?).is_some(),
            "max_calls" => contract.max_calls.replace(narrow_value()?).is_some(),
            "max_allocations" => contract.max_allocations.replace(narrow_value()?).is_some(),
            "max_allocation_bytes" => contract.max_allocation_bytes.replace(value).is_some(),
            "max_stack_bytes" => contract.max_stack_bytes.replace(value).is_some(),
            "max_io_bytes" => contract.max_io_bytes.replace(value).is_some(),
            _ => return Err(self.invalid_bounded("unknown budget field", span)),
        };
        if duplicate {
            return Err(self.invalid_bounded("duplicate budget field", span));
        }
        Ok(())
    }

    fn invalid_bounded(&self, reason: &str, span: crate::lexer::SourceSpan) -> ParseError {
        ParseError {
            code: ParseErrorCode::InvalidBoundedLimit,
            kind: ParseErrorKind::InvalidBoundedLimit { reason: reason.to_owned() },
            span,
        }
    }
}

fn bounded_integer(
    token: crate::lexer::Token,
    name: &str,
) -> Result<(u64, crate::lexer::SourceSpan), ParseError> {
    let span = token.span;
    let valid = match token.kind {
        TokenKind::Integer { value, suffix: Some(suffix) } => {
            let expected = name.ends_with("bytes") || name == "max_io_bytes";
            let expected_suffix = if expected { "u64" } else { "u32" };
            (suffix == expected_suffix).then(|| value.parse::<u64>().ok()).flatten()
        }
        _ => None,
    };
    valid.map(|value| (value, span)).ok_or_else(|| ParseError {
        code: ParseErrorCode::InvalidBoundedLimit,
        kind: ParseErrorKind::InvalidBoundedLimit {
            reason: format!("`{name}` must be a non-negative bounded integer literal"),
        },
        span,
    })
}
