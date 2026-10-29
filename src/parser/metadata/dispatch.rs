use super::super::{ParseError, ParseErrorCode, ParseErrorKind, Parser, identifier_text};
use crate::ast::MetaAttribute;
use crate::lexer::TokenKind;

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
            name => Err(ParseError {
                code: ParseErrorCode::UnknownMetadata,
                kind: ParseErrorKind::UnknownMetadata { name: name.to_owned() },
                span: attribute.span,
            }),
        }
    }
}
