use crate::diagnostics::{render_lex_error, render_parse_error, render_semantic_error};
use crate::lexer::scan;
use crate::modules::{ModuleError, ModuleResolver, parse_module, resolve_imports};
use crate::parser::parse;
use crate::semantic::filter_program_for_target;
use std::fs;

use super::conformance::{ConformanceMode, parse_strict_option};

pub(super) fn check_command(mut arguments: impl Iterator<Item = String>) -> i32 {
    let options = match parse_check_options(&mut arguments) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("error: {error}");
            return 2;
        }
    };
    let explicit_input = options.input;
    let configuration = match super::input::configuration_for_input_with_mode(
        explicit_input.as_deref(),
        options.mode,
    ) {
        Ok(configuration) => configuration,
        Err(error) => {
            eprintln!("error: {error}");
            return 1;
        }
    };
    let input_path = super::input::entry_path(&configuration, explicit_input.as_deref());
    check_input(&configuration, &input_path, explicit_input.is_some(), options.mode)
}

struct CheckOptions {
    input: Option<String>,
    mode: ConformanceMode,
}

fn parse_check_options(arguments: impl Iterator<Item = String>) -> Result<CheckOptions, String> {
    let mut input = None;
    let mut mode = ConformanceMode::Standard;
    for argument in arguments {
        if parse_strict_option(&argument, &mut mode)? {
            continue;
        }
        if input.is_none() && !argument.starts_with('-') {
            input = Some(argument);
            continue;
        }
        return Err(format!("unexpected check argument `{argument}`"));
    }
    Ok(CheckOptions { input, mode })
}

fn check_input(
    configuration: &crate::configuration::CompilerConfiguration,
    input_path: &std::path::Path,
    explicit: bool,
    mode: ConformanceMode,
) -> i32 {
    if explicit
        && let Some(module_path) =
            super::input::module_path_for_file(configuration.source_root(), input_path)
    {
        return check_module(configuration, input_path, &module_path, mode);
    }
    check_source(configuration, input_path, mode)
}

fn check_source(
    configuration: &crate::configuration::CompilerConfiguration,
    input_path: &std::path::Path,
    mode: ConformanceMode,
) -> i32 {
    let input = super::input::source_path(input_path);
    let source = match fs::read_to_string(&input) {
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
    let program = match resolve_imports(
        &program,
        &ModuleResolver::with_dependencies(
            configuration.source_root(),
            configuration.dependency_roots(),
        ),
    ) {
        Ok(program) => program,
        Err(error) => {
            eprintln!("error: cannot resolve imports for `{input}`: {error}");
            return 1;
        }
    };
    let program = filter_program_for_target(&program, configuration.target());
    match crate::semantic::analyze(&program) {
        Ok(_) => {
            if mode.is_strict() {
                println!("checked `{input}` successfully (strict)");
            } else {
                println!("checked `{input}` successfully");
            }
            0
        }
        Err(error) => {
            eprintln!("{input}: {}", render_semantic_error(&source, &error));
            1
        }
    }
}

fn check_module(
    configuration: &crate::configuration::CompilerConfiguration,
    input_path: &std::path::Path,
    module_path: &str,
    mode: ConformanceMode,
) -> i32 {
    let resolver = ModuleResolver::with_dependencies(
        configuration.source_root(),
        configuration.dependency_roots(),
    );
    let program = match parse_module(&resolver, module_path) {
        Ok(program) => program,
        Err(error) => return report_module_error(module_path, error),
    };
    let program = match resolve_imports(&program, &resolver) {
        Ok(program) => program,
        Err(error) => return report_module_error(module_path, error),
    };
    let program = filter_program_for_target(&program, configuration.target());
    match crate::semantic::analyze(&program) {
        Ok(_) => {
            if mode.is_strict() {
                println!("checked `{}` successfully (strict)", input_path.display());
            } else {
                println!("checked `{}` successfully", input_path.display());
            }
            0
        }
        Err(error) => {
            eprintln!("{}: {}", input_path.display(), render_semantic_error("", &error));
            1
        }
    }
}

fn report_module_error(module_path: &str, error: ModuleError) -> i32 {
    eprintln!("error: cannot check module `{module_path}`: {error}");
    1
}
