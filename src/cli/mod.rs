use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::codegen::{emit_program_object, link_object};
use crate::diagnostics::{render_lex_error, render_parse_error, render_semantic_error};
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
    let mut emit = EmitKind::Object;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "-o" => {
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
            "--emit" => {
                let Some(kind) = arguments.next() else {
                    eprintln!("error: missing value after `--emit`");
                    return 2;
                };
                emit = match kind.as_str() {
                    "obj" => EmitKind::Object,
                    "exe" => EmitKind::Executable,
                    _ => {
                        eprintln!("error: unsupported emission kind `{kind}`");
                        return 2;
                    }
                };
            }
            _ => {
                eprintln!("error: unexpected build argument `{argument}`");
                print_usage();
                return 2;
            }
        }
    }
    build_file(&input, output.as_deref().map(Path::new), emit)
}

#[derive(Clone, Copy)]
enum EmitKind {
    Object,
    Executable,
}

fn validate_entry(
    program: &crate::ast::Program,
    symbol: &str,
    emit: EmitKind,
) -> Result<(), String> {
    if matches!(emit, EmitKind::Executable)
        && let crate::ast::TopLevelDecl::Verb(verb) = &program.declarations[0]
        && !verb.params.is_empty()
    {
        return Err(format!("executable entry verb `{symbol}` cannot have parameters"));
    }
    Ok(())
}

fn build_file(input: &str, output: Option<&Path>, emit: EmitKind) -> i32 {
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
    if let Err(error) = validate_entry(&program, symbol, emit) {
        eprintln!("error: {error}");
        return 1;
    }
    let bytes = match emit_program_object(&program, symbol) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("error: cannot build `{input}`: {error}");
            return 1;
        }
    };
    let output = output.map(PathBuf::from).unwrap_or_else(|| default_output(input, emit));
    if let Err(error) = write_artifact(input, &output, bytes, emit) {
        eprintln!("error: {error}");
        return 1;
    }
    println!("built `{}`", output.display());
    0
}

fn write_artifact(
    input: &str,
    output: &Path,
    bytes: Vec<u8>,
    emit: EmitKind,
) -> Result<(), String> {
    let object = matches!(emit, EmitKind::Executable).then(|| output.with_extension("o"));
    let object_path = object.as_deref().unwrap_or(output);
    fs::write(object_path, bytes)
        .map_err(|error| format!("cannot write `{}`: {error}", object_path.display()))?;
    if let Some(object_path) = object {
        let result = link_object(&object_path, output)
            .map_err(|error| format!("cannot link `{input}`: {error}"));
        let _ = fs::remove_file(object_path);
        result
    } else {
        Ok(())
    }
}

fn default_output(input: &str, emit: EmitKind) -> PathBuf {
    let path = Path::new(input);
    if matches!(emit, EmitKind::Object) {
        path.with_extension("o")
    } else {
        path.with_file_name(path.file_stem().unwrap_or_default())
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
    eprintln!("usage: actus <parse|check|fmt> [--check] <file.act>");
    eprintln!("       actus build <file.act> [--emit obj|exe] [-o <output>]");
}
