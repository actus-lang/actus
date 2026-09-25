use std::env;
use std::fs;

use crate::diagnostics::{render_lex_error, render_parse_error, render_semantic_error};
use crate::lexer::scan;
use crate::parser::parse;

mod build;
mod check;
mod fmt;
mod lsp;
mod project;
mod publish;
mod run;
mod test_runner;

pub fn run() -> i32 {
    run_with_args(env::args().skip(1))
}

pub fn run_with_args(mut arguments: impl Iterator<Item = String>) -> i32 {
    let Some(command) = arguments.next() else {
        print_usage();
        return 2;
    };
    if command == "build" {
        return build::build_command(arguments);
    }
    if command == "run" {
        return run::run_command(arguments);
    }
    if command == "check" {
        return check::check_command(arguments);
    }
    if command == "new" {
        return project::new_command(arguments);
    }
    if command == "init" {
        return project::init_command(arguments);
    }
    if command == "test" {
        return test_runner::test_command(arguments);
    }
    if command == "fmt" {
        return fmt::fmt_command(arguments);
    }
    if command == "lsp" {
        return lsp::lsp_command(arguments);
    }
    if command == "publish" {
        return publish::publish_command(arguments);
    }

    let Some(first_argument) = arguments.next() else {
        eprintln!("error: missing input file");
        print_usage();
        return 2;
    };

    let path = first_argument;

    if arguments.next().is_some() {
        eprintln!("error: unexpected extra argument");
        return 2;
    }

    match command.as_str() {
        "parse" => parse_file(&path, true),
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
        Ok(program) => match crate::semantic::analyze(&program) {
            Ok(_) => {
                println!("checked `{path}` successfully");
                0
            }
            Err(error) => {
                eprintln!("{path}: {}", render_semantic_error(&source, &error));
                1
            }
        },
        Err(error) => {
            eprintln!("{path}: {}", render_parse_error(&source, &error));
            1
        }
    }
}

fn print_usage() {
    eprintln!("usage: actus <new|init|check|parse|run|test|fmt|lsp|publish> [options] [file.act]");
    eprintln!(
        "       actus build <file.act> [--release|--profile <name>] [--emit obj|exe] [-o <output>]"
    );
}
