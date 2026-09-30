use crate::diagnostics::{
    lex_diagnostic, module_diagnostic, parse_diagnostic, render_diagnostic, semantic_diagnostic,
};
use crate::lexer::scan;
use crate::modules::{ModuleError, ModuleResolver, parse_module, resolve_imports};
use crate::parser::parse;
use crate::semantic::filter_program_for_target;
use std::fs;

use super::conformance::{ConformanceMode, parse_strict_option, validate_source_limits};
use super::diagnostics::{report_diagnostic, report_diagnostics};

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
    let input = input_path.to_path_buf();
    let Some(source) = read_check_source(&input) else { return 1 };
    if !validate_source_limits(&input, &source, mode, configuration) {
        return 1;
    }
    let Some(program) = parse_check_source(&input, &source) else { return 1 };
    let Some(program) = resolve_check_program(configuration, &input, &source, program) else {
        return 1;
    };
    analyze_check_program(&input, &source, configuration, mode, program)
}

fn read_check_source(input: &std::path::Path) -> Option<String> {
    match fs::read_to_string(input) {
        Ok(source) => Some(source),
        Err(error) => {
            eprintln!("error: cannot read `{}`: {error}", input.display());
            None
        }
    }
}

fn parse_check_source(input: &std::path::Path, source: &str) -> Option<crate::ast::Program> {
    let (tokens, lex_errors) = scan(source);
    if !lex_errors.is_empty() {
        report_diagnostics(input, source, lex_errors.iter().map(lex_diagnostic).collect());
        return None;
    }
    match parse(tokens) {
        Ok(program) => Some(program),
        Err(error) => {
            report_diagnostic(input, source, parse_diagnostic(&error));
            None
        }
    }
}

fn resolve_check_program(
    configuration: &crate::configuration::CompilerConfiguration,
    input: &std::path::Path,
    source: &str,
    program: crate::ast::Program,
) -> Option<crate::ast::Program> {
    let resolver = ModuleResolver::with_dependencies(
        configuration.source_root(),
        configuration.dependency_roots(),
    );
    match resolve_imports(&program, &resolver) {
        Ok(program) => Some(program),
        Err(error) => {
            let diagnostic = module_diagnostic(&error);
            eprintln!("{}: {}", input.display(), render_diagnostic(source, &diagnostic));
            None
        }
    }
}

fn analyze_check_program(
    input: &std::path::Path,
    source: &str,
    configuration: &crate::configuration::CompilerConfiguration,
    mode: ConformanceMode,
    program: crate::ast::Program,
) -> i32 {
    let program = filter_program_for_target(&program, configuration.target());
    match crate::semantic::analyze(&program) {
        Ok(_) => {
            if mode.is_strict() {
                println!("checked `{}` successfully (strict)", input.display());
            } else {
                println!("checked `{}` successfully", input.display());
            }
            0
        }
        Err(error) => {
            report_diagnostic(input, source, semantic_diagnostic(&error));
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
            report_diagnostic(input_path, "", semantic_diagnostic(&error));
            1
        }
    }
}

fn report_module_error(module_path: &str, error: ModuleError) -> i32 {
    let diagnostic = module_diagnostic(&error);
    eprintln!("{module_path}: {}", render_diagnostic("", &diagnostic));
    1
}
