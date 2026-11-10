use crate::ast::Program;
use crate::lexer::Token;

use super::state::{ParseError, Parser};

pub fn parse(tokens: Vec<Token>) -> Result<Program, ParseError> {
    Parser::new(tokens).parse()
}
