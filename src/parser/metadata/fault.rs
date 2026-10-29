use super::super::{ParseError, Parser};
use crate::ast::MetaAttribute;

impl Parser {
    pub(super) fn parse_fault_metadata(&mut self) -> Result<Vec<MetaAttribute>, ParseError> {
        self.expect_simple(crate::lexer::TokenKind::LeftParen, "`(` after `fault`")?;
        let fault = super::syntax::parse_fault_name(self.advance_required("fault name string")?)?;
        self.expect_simple(crate::lexer::TokenKind::RightParen, "`)` after fault name")?;
        let _ = self.match_simple(crate::lexer::TokenKind::Semicolon);
        Ok(vec![MetaAttribute::Fault(fault)])
    }
}
