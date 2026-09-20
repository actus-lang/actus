use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::codegen::emit_program_object;
use crate::diagnostics::{render_lex_error, render_parse_error};
use crate::formatter::format_program;
use crate::lexer::scan;
use crate::parser::parse;

pub fn run() -> i32 {
    run_with_args(env::args().skip(1))
}

pub fn run_with_args(mut arguments: impl Iterator<Item = String>) -> i32 {
    let Some(command) = arguments.next() else {
        print_usage();
        return 2;
    };
    if command == "build" {
        return build_command(arguments);
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

fn build_command(mut arguments: impl Iterator<Item = String>) -> i32 {
    let Some(input) = arguments.next() else {
        eprintln!("error: missing input file");
        print_usage();
        return 2;
    };
    let mut output = None;
    while let Some(argument) = arguments.next() {
        if argument != "-o" {
            eprintln!("error: unexpected build argument `{argument}`");
            print_usage();
            return 2;
        }
        if output.is_some() {
            eprintln!("error: duplicate output option");
            return 2;
        }
        output = arguments.next();
        if output.is_none() {
            eprintln!("error: missing output path after `-o`");
            return 2;
        }
    }
    build_file(&input, output.as_deref().map(Path::new))
}

fn build_file(input: &str, output: Option<&Path>) -> i32 {
    let source = match fs::read_to_string(input) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("error: cannot read `{input}`: {error}");
            return 1;
        }
    };
    let (tokens, lex_errors) = scan(&source);
    if !lex_errors.is_empty() {
        for error in &lex_errors {
            eprintln!("{input}: {}", render_lex_error(&source, error));
        }
        return 1;
    }
    let program = match parse(tokens) {
        Ok(program) => program,
        Err(error) => {
            eprintln!("{input}: {}", render_parse_error(&source, &error));
            return 1;
        }
    };
    let symbol = match program.declarations.first() {
        Some(crate::ast::TopLevelDecl::Verb(verb)) => verb.name.as_str(),
        None => {
            eprintln!("error: `{input}` contains no verb declarations");
            return 1;
        }
    };
    let bytes = match emit_program_object(&program, symbol) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("error: cannot build `{input}`: {error}");
            return 1;
        }
    };
    let output = output.map(PathBuf::from).unwrap_or_else(|| default_output(input));
    if let Err(error) = fs::write(&output, bytes) {
        eprintln!("error: cannot write `{}`: {error}", output.display());
        return 1;
    }
    println!("built `{}`", output.display());
    0
}

fn default_output(input: &str) -> PathBuf {
    let path = Path::new(input);
    path.with_extension("o")
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
    eprintln!("usage: actus <parse|check|fmt> [--check] <file.act>");
    eprintln!("       actus build <file.act> [-o <output.o>]");
}
