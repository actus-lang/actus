use std::env;
use std::fs;

use crate::diagnostics::{render_lex_error, render_parse_error};
use crate::lexer::scan;
use crate::parser::parse;

pub fn run() -> i32 {
    let mut arguments = env::args().skip(1);
    let Some(command) = arguments.next() else {
        print_usage();
        return 2;
    };
    let Some(path) = arguments.next() else {
        eprintln!("error: missing input file");
        print_usage();
        return 2;
    };

    if arguments.next().is_some() {
        eprintln!("error: unexpected extra argument");
        return 2;
    }

    match command.as_str() {
        "parse" => parse_file(&path, true),
        "check" => parse_file(&path, false),
        _ => {
            eprintln!("error: unknown command `{command}`");
            print_usage();
            2
        }
    }
}

fn parse_file(path: &str, print_ast: bool) -> i32 {
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("error: cannot read `{path}`: {error}");
            return 1;
        }
    };

    let (tokens, lex_errors) = scan(&source);
    if !lex_errors.is_empty() {
        for error in &lex_errors {
            eprintln!("{path}: {}", render_lex_error(&source, error));
        }
        return 1;
    }

    match parse(tokens) {
        Ok(program) if print_ast => {
            println!("{program:#?}");
            0
        }
        Ok(_) => {
            println!("checked `{path}` successfully");
            0
        }
        Err(error) => {
            eprintln!("{path}: {}", render_parse_error(&source, &error));
            1
        }
    }
}

fn print_usage() {
    eprintln!("usage: actus <parse|check> <file.act>");
}
