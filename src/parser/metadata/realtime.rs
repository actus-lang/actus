use crate::ast::MetaAttribute;
use crate::parser::{ParseError, Parser};

impl Parser {
    pub(super) fn parse_realtime_metadata(&mut self) -> Result<Vec<MetaAttribute>, ParseError> {
        Ok(vec![MetaAttribute::Realtime])
    }
}
