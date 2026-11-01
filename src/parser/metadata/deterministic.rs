use super::super::{ParseError, Parser};
use crate::ast::MetaAttribute;

impl Parser {
    pub(super) fn parse_deterministic_metadata(
        &mut self,
    ) -> Result<Vec<MetaAttribute>, ParseError> {
        Ok(vec![MetaAttribute::Deterministic])
    }
}
