use crate::ast::MetaAttribute;
use crate::lexer::TokenKind;
use crate::parser::{ParseError, Parser};

impl Parser {
    pub(super) fn parse_test_metadata(&mut self) -> Result<Vec<MetaAttribute>, ParseError> {
        let _ = self.match_simple(TokenKind::Semicolon);
        Ok(vec![MetaAttribute::Test])
    }
}
