use crate::ast::MetaAttribute;
use crate::lexer::TokenKind;
use crate::parser::state::identifier_text;
use crate::parser::{ParseError, ParseErrorCode, ParseErrorKind, Parser};

impl Parser {
    pub(in crate::parser) fn parse_metadata(&mut self) -> Result<Vec<MetaAttribute>, ParseError> {
        self.expect_keyword(TokenKind::Meta, "`meta`")?;
        let attribute = self.take_identifier("metadata name")?;
        match identifier_text(&attribute.kind).as_str() {
            "test" => self.parse_test_metadata(),
            "target" => self.parse_target_metadata(),
            "fault" => self.parse_fault_metadata(),
            "limitless" => self.parse_limitless_metadata(),
            "must_use" => self.parse_must_use_metadata(),
            "effect" => self.parse_effect_metadata(),
            "deterministic" => self.parse_deterministic_metadata(),
            "realtime" => self.parse_realtime_metadata(),
            "bounded" => self.parse_bounded_metadata(),
            "stack_limit" => self.parse_stack_limit_metadata(),
            "capability" => self.parse_capability_metadata(),
            "repr" => self.parse_repr_metadata(),
            "volatile" => Ok(vec![MetaAttribute::Volatile]),
            "zero_on_drop" => Ok(vec![MetaAttribute::ZeroOnDrop]),
            name => Err(ParseError {
                code: ParseErrorCode::UnknownMetadata,
                kind: ParseErrorKind::UnknownMetadata { name: name.to_owned() },
                span: attribute.span,
            }),
        }
    }
}
