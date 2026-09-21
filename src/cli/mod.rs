use std::env;
use std::fs;

use crate::configuration::CompilerConfiguration;
use crate::diagnostics::{render_lex_error, render_parse_error, render_semantic_error};
use crate::formatter::format_program;
use crate::lexer::scan;
use crate::parser::parse;

mod build;
mod run;

pub fn run() -> i32 {
    run_with_args(env::args().skip(1))
}

pub fn run_with_args(mut arguments: impl Iterator<Item = String>) -> i32 {
    let configuration = match CompilerConfiguration::from_current_manifest() {
        Ok(configuration) => configuration,
        Err(error) => {
            eprintln!("error: {error}");
            return 1;
        }
    };
    let Some(command) = arguments.next() else {
        print_usage();
        return 2;
    };
    if command == "build" {
        return build::build_command(arguments, &configuration);
    }
    if command == "run" {
        return run::run_command(arguments, &configuration);
    }

    let Some(first_argument) = arguments.next() else {
        eprintln!("error: missing input file");
        print_usage();
        return 2;
    };

    let (check_only, path) = if command == "fmt" && first_argument == "--check" {
        let Some(path) = arguments.next() else {
            eprintln!("error: missing input file");
            print_usage();
            return 2;
        };
        (true, path)
    } else {
        (false, first_argument)
    };

    if arguments.next().is_some() {
        eprintln!("error: unexpected extra argument");
        return 2;
    }

    match command.as_str() {
        "parse" => parse_file(&path, true),
        "check" => parse_file(&path, false),
        "fmt" => format_file(&path, check_only),
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

fn format_file(path: &str, check_only: bool) -> i32 {
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

    let program = match parse(tokens) {
        Ok(program) => program,
        Err(error) => {
            eprintln!("{path}: {}", render_parse_error(&source, &error));
            return 1;
        }
    };
    let formatted = format_program(&program);

    if check_only {
        if formatted == source {
            println!("{path} is formatted");
            0
        } else {
            eprintln!("{path} is not formatted");
            1
        }
    } else if let Err(error) = fs::write(path, formatted) {
        eprintln!("error: cannot write `{path}`: {error}");
        1
    } else {
        println!("formatted `{path}`");
        0
    }
}

fn print_usage() {
    eprintln!("usage: actus <parse|check|run|fmt> [--check] <file.act>");
    eprintln!("       actus build <file.act> [--emit obj|exe] [-o <output>]");
}
