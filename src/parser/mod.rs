#[path = "parser.rs"]
mod implementation;

pub use implementation::{ParseError, ParseErrorCode, ParseErrorKind, Parser};

use crate::ast::Program;
use crate::lexer::Token;

pub fn parse(tokens: Vec<Token>) -> Result<Program, ParseError> {
    Parser::new(tokens).parse()
}
