use std::fs;
use std::path::{Path, PathBuf};

use crate::codegen::{emit_program_object_with_configuration, link_object};
use crate::configuration::{CompilerConfiguration, HOSTED_ENTRY_SYMBOL};
use crate::diagnostics::{render_lex_error, render_parse_error};
use crate::lexer::scan;
use crate::parser::parse;

#[derive(Clone, Copy)]
pub(super) enum EmitKind {
    Object,
    Executable,
}

pub(super) fn build_command(
    mut arguments: impl Iterator<Item = String>,
    configuration: &CompilerConfiguration,
) -> i32 {
    let Some(input) = arguments.next() else {
        eprintln!("error: missing input file");
        super::print_usage();
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
                super::print_usage();
                return 2;
            }
        }
    }
    build_file(&input, output.as_deref().map(Path::new), emit, configuration)
}

pub(super) fn build_file(
    input: &str,
    output: Option<&Path>,
    emit: EmitKind,
    configuration: &CompilerConfiguration,
) -> i32 {
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
    let Some(fallback_symbol) = first_defined_verb(&program) else {
        eprintln!("error: `{input}` contains no verb declarations");
        return 1;
    };
    let symbol = configuration.entry_symbol().unwrap_or(fallback_symbol).to_owned();
    if let Err(error) = validate_entry(&program, &symbol, emit) {
        eprintln!("error: {error}");
        return 1;
    }
    let bytes = match emit_program_object_with_configuration(
        &program,
        &symbol,
        configuration.native_backend(),
    ) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("error: cannot build `{input}`: {error}");
            return 1;
        }
    };
    let output = output.map(PathBuf::from).unwrap_or_else(|| default_output(input, emit));
    if let Err(error) = write_artifact(input, &output, bytes, emit, configuration) {
        eprintln!("error: {error}");
        return 1;
    }
    println!("built `{}`", output.display());
    0
}

fn first_defined_verb(program: &crate::ast::Program) -> Option<&str> {
    program.declarations.iter().find_map(|declaration| match declaration {
        crate::ast::TopLevelDecl::Verb(verb) => Some(verb.name.as_str()),
        crate::ast::TopLevelDecl::ExternalVerb(_) | crate::ast::TopLevelDecl::Struct(_) => None,
    })
}

fn validate_entry(
    program: &crate::ast::Program,
    symbol: &str,
    emit: EmitKind,
) -> Result<(), String> {
    let Some(crate::ast::TopLevelDecl::Verb(verb)) = program.declarations.iter().find(|decl| {
        matches!(decl, crate::ast::TopLevelDecl::Verb(candidate) if candidate.name == symbol)
    }) else {
        return Err(format!("entry verb `{symbol}` was not found"));
    };
    if matches!(emit, EmitKind::Executable) && symbol != HOSTED_ENTRY_SYMBOL {
        return Err(format!(
            "hosted executables require entry verb `{HOSTED_ENTRY_SYMBOL}`; custom entry points are available for object emission only"
        ));
    }
    if matches!(emit, EmitKind::Executable) && !verb.params.is_empty() {
        return Err(format!("executable entry verb `{symbol}` cannot have parameters"));
    }
    Ok(())
}

fn write_artifact(
    input: &str,
    output: &Path,
    bytes: Vec<u8>,
    emit: EmitKind,
    configuration: &CompilerConfiguration,
) -> Result<(), String> {
    let object = matches!(emit, EmitKind::Executable).then(|| output.with_extension("o"));
    let object_path = object.as_deref().unwrap_or(output);
    fs::write(object_path, bytes)
        .map_err(|error| format!("cannot write `{}`: {error}", object_path.display()))?;
    if let Some(object_path) = object {
        let result = link_object(&object_path, output, configuration)
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
