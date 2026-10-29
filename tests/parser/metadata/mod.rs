use actus::ast::Program;
use actus::lexer::scan;
use actus::parser::{ParseError, parse};

fn parse_source(source: &str) -> Program {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    parse(tokens).expect("source should parse")
}

fn parse_error(source: &str) -> ParseError {
    let (tokens, errors) = scan(source);
    assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
    parse(tokens).expect_err("source should be rejected")
}

mod fault;
mod limitless;
mod must_use;
mod syntax;
mod target;
mod test;
