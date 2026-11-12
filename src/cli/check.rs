use crate::SharedSourceMap;
use crate::diagnostics::{
    lex_diagnostic, module_diagnostic, parse_diagnostic, render_diagnostic, semantic_diagnostic,
};
use crate::modules::{ModuleError, ModuleResolver, parse_module, resolve_imports};
use crate::parser::parse;
use crate::semantic::filter_program_for_target_checked;
use std::fs;

use super::conformance::{ConformanceMode, parse_strict_option, validate_source_limits};
use super::diagnostics::{report_mapped_diagnostic, report_mapped_diagnostics};

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
    let source_map = SharedSourceMap::new();
    if !validate_source_limits(&input, &source, mode, configuration) {
        return 1;
    }
    let Some(program) = parse_check_source(&input, &source_map, &source) else { return 1 };
    let Some(program) =
        resolve_check_program(configuration, &input, &source, program, source_map.clone())
    else {
        return 1;
    };
    analyze_check_program(&input, &source, &source_map, configuration, mode, program)
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

fn parse_check_source(
    input: &std::path::Path,
    source_map: &SharedSourceMap,
    source: &str,
) -> Option<crate::ast::Program> {
    let (tokens, lex_errors) = source_map.scan(input, source);
    if !lex_errors.is_empty() {
        report_mapped_diagnostics(
            source_map,
            input,
            source,
            lex_errors.iter().map(lex_diagnostic).collect(),
        );
        return None;
    }
    match parse(tokens) {
        Ok(program) => Some(program),
        Err(error) => {
            report_mapped_diagnostic(source_map, input, source, parse_diagnostic(&error));
            None
        }
    }
}

fn resolve_check_program(
    configuration: &crate::configuration::CompilerConfiguration,
    input: &std::path::Path,
    source: &str,
    program: crate::ast::Program,
    source_map: SharedSourceMap,
) -> Option<crate::ast::Program> {
    let resolver = ModuleResolver::with_dependencies_and_runtime(
        configuration.source_root(),
        configuration.dependency_roots(),
        configuration.runtime_source_root(),
        configuration.runtime_module_roots(),
    )
    .with_source_map(source_map.clone());
    match resolve_imports(&program, &resolver) {
        Ok(program) => Some(program),
        Err(error) => {
            let diagnostic = module_diagnostic(&error);
            report_mapped_diagnostic(&source_map, input, source, diagnostic);
            None
        }
    }
}

fn analyze_check_program(
    input: &std::path::Path,
    source: &str,
    source_map: &SharedSourceMap,
    configuration: &crate::configuration::CompilerConfiguration,
    mode: ConformanceMode,
    program: crate::ast::Program,
) -> i32 {
    let program = match filter_program_for_target_checked(&program, configuration.target()) {
        Ok(program) => program,
        Err(error) => {
            report_mapped_diagnostic(source_map, input, source, semantic_diagnostic(&error));
            return 1;
        }
    };
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
            report_mapped_diagnostic(source_map, input, source, semantic_diagnostic(&error));
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
    let source_map = SharedSourceMap::new();
    let resolver = ModuleResolver::with_dependencies_and_runtime(
        configuration.source_root(),
        configuration.dependency_roots(),
        configuration.runtime_source_root(),
        configuration.runtime_module_roots(),
    )
    .with_source_map(source_map.clone());
    let program = match parse_module(&resolver, module_path) {
        Ok(program) => program,
        Err(error) => return report_module_error(module_path, error, &source_map),
    };
    let program = match resolve_imports(&program, &resolver) {
        Ok(program) => program,
        Err(error) => return report_module_error(module_path, error, &source_map),
    };
    let program = match filter_program_for_target_checked(&program, configuration.target()) {
        Ok(program) => program,
        Err(error) => {
            report_mapped_diagnostic(&source_map, input_path, "", semantic_diagnostic(&error));
            return 1;
        }
    };
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
            report_mapped_diagnostic(&source_map, input_path, "", semantic_diagnostic(&error));
            1
        }
    }
}

fn report_module_error(module_path: &str, error: ModuleError, source_map: &SharedSourceMap) -> i32 {
    let diagnostic = module_diagnostic(&error);
    if let Some((path, source, diagnostic)) = source_map.lock().localize(&diagnostic) {
        eprintln!("{}: {}", path.display(), render_diagnostic(&source, &diagnostic));
    } else {
        eprintln!("{module_path}: {}", render_diagnostic("", &diagnostic));
    }
    1
}
