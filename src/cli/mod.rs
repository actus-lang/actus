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
    if is_help_flag(&command) {
        print_help();
        return 0;
    }
    let remaining: Vec<String> = arguments.collect();
    if remaining.first().is_some_and(|argument| is_help_flag(argument)) {
        return print_command_help(&command);
    }
    dispatch_command(&command, remaining)
}

fn dispatch_command(command: &str, arguments: Vec<String>) -> i32 {
    let arguments = arguments.into_iter();
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

    if command != "parse" {
        eprintln!("error: unknown command `{command}`");
        print_usage();
        return 2;
    }
    parse_command(arguments)
}

fn parse_command(mut arguments: impl Iterator<Item = String>) -> i32 {
    let Some(path) = arguments.next() else {
        eprintln!("error: missing input file");
        print_usage();
        return 2;
    };
    if arguments.next().is_some() {
        eprintln!("error: unexpected extra argument");
        return 2;
    }
    parse_file(&path, true)
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
    eprintln!("{}", usage_text());
}

fn print_help() {
    println!("{}", usage_text());
    println!();
    println!("Run `actus <command> --help` for command-specific options.");
}

fn print_command_help(command: &str) -> i32 {
    let Some(text) = command_help_text(command) else {
        eprintln!("error: unknown command `{command}`");
        print_usage();
        return 2;
    };
    println!("{text}");
    0
}

fn is_help_flag(argument: &str) -> bool {
    matches!(argument, "--help" | "-h")
}

fn usage_text() -> &'static str {
    "usage: actus <new|init|check|parse|build|run|test|fmt|lsp|publish> [options] [file.act]"
}

fn command_help_text(command: &str) -> Option<&'static str> {
    match command {
        "new" => {
            Some("usage: actus new <name> [--no-git|--vcs none]\n\nCreate a new Actus project.")
        }
        "init" => Some(
            "usage: actus init [--no-git|--vcs none]\n\nInitialize the current directory as an Actus project.",
        ),
        "check" => Some(
            "usage: actus check [file.act]\n\nParse, resolve, and validate an Actus project without code generation.",
        ),
        "parse" => Some("usage: actus parse <file.act>\n\nParse a source file and print its AST."),
        "build" => Some(
            "usage: actus build [file.act] [--release|--profile <name>] [--emit obj|exe] [-o <output>]\n\nBuild an Actus source file or the project entry from Arca.toml.",
        ),
        "run" => Some(
            "usage: actus run [file.act] [--release|--profile <name>] [-- program-args...]\n\nBuild and execute an Actus program once.",
        ),
        "test" => Some("usage: actus test [path]\n\nCollect and run Actus meta tests."),
        "fmt" => Some(
            "usage: actus fmt [path] [--check]\n\nFormat Actus source files with canonical rules.",
        ),
        "lsp" => Some("usage: actus lsp\n\nServe the Language Server Protocol over stdio."),
        "publish" => Some(
            "usage: actus publish [path]\n\nValidate and publish a local deterministic package archive.",
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{command_help_text, run_with_args, usage_text};

    #[test]
    fn top_level_help_returns_success() {
        assert_eq!(run_with_args(vec!["--help".to_owned()].into_iter()), 0);
        assert_eq!(run_with_args(vec!["-h".to_owned()].into_iter()), 0);
    }

    #[test]
    fn every_command_has_help_text() {
        for command in
            ["new", "init", "check", "parse", "build", "run", "test", "fmt", "lsp", "publish"]
        {
            assert!(command_help_text(command).is_some(), "missing help for {command}");
        }
        assert!(usage_text().contains("build"));
    }

    #[test]
    fn subcommand_help_returns_success() {
        assert_eq!(run_with_args(vec!["build".to_owned(), "--help".to_owned()].into_iter()), 0);
    }
}
