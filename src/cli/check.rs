use crate::SharedSourceMap;
use crate::diagnostics::{module_diagnostic, render_diagnostic};
use crate::modules::{
    ModuleError, ModuleResolver, build_compilation_plan, parse_module, validate_compilation_plan,
};
use crate::session::CompilationSession;
use std::fs;

use super::conformance::{ConformanceMode, parse_strict_option, validate_source_limits};
use super::diagnostics::{report_frontend_error, report_mapped_diagnostic};

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
    let session = CompilationSession::new();
    if !validate_source_limits(&input, &source, mode, configuration) {
        return 1;
    }
    let Some(program) = parse_check_source(&input, &session, &source) else { return 1 };
    let Some(plan) = resolve_check_program(configuration, &input, &source, program, &session)
    else {
        return 1;
    };
    analyze_check_program(&input, &source, session.source_map(), configuration, mode, plan)
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
    session: &CompilationSession,
    source: &str,
) -> Option<crate::ast::Program> {
    match session.parse(input, source).program() {
        Ok(program) => Some(program),
        Err(error) => {
            report_frontend_error(session.source_map(), input, source, &error);
            None
        }
    }
}

fn resolve_check_program(
    configuration: &crate::configuration::CompilerConfiguration,
    input: &std::path::Path,
    source: &str,
    program: crate::ast::Program,
    session: &CompilationSession,
) -> Option<crate::modules::ModuleCompilationPlan> {
    let resolver = ModuleResolver::with_dependencies_and_runtime(
        configuration.source_root(),
        configuration.dependency_roots(),
        configuration.runtime_source_root(),
        configuration.runtime_module_roots(),
    )
    .with_session(session.clone());
    match build_compilation_plan(&program, &resolver) {
        Ok(plan) => Some(plan),
        Err(error) => {
            let diagnostic = module_diagnostic(&error);
            report_mapped_diagnostic(session.source_map(), input, source, diagnostic);
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
    plan: crate::modules::ModuleCompilationPlan,
) -> i32 {
    if let Err(error) = validate_compilation_plan(&plan, configuration.target()) {
        report_mapped_diagnostic(source_map, input, source, module_diagnostic(&error));
        return 1;
    }
    if mode.is_strict() {
        println!("checked `{}` successfully (strict)", input.display());
    } else {
        println!("checked `{}` successfully", input.display());
    }
    0
}

fn check_module(
    configuration: &crate::configuration::CompilerConfiguration,
    input_path: &std::path::Path,
    module_path: &str,
    mode: ConformanceMode,
) -> i32 {
    let session = CompilationSession::new();
    let source_map = session.source_map();
    let resolver = ModuleResolver::with_dependencies_and_runtime(
        configuration.source_root(),
        configuration.dependency_roots(),
        configuration.runtime_source_root(),
        configuration.runtime_module_roots(),
    )
    .with_session(session.clone());
    let program = match parse_module(&resolver, module_path) {
        Ok(program) => program,
        Err(error) => return report_module_error(module_path, error, source_map),
    };
    let plan = match build_compilation_plan(&program, &resolver) {
        Ok(plan) => plan,
        Err(error) => return report_module_error(module_path, error, source_map),
    };
    if let Err(error) = validate_compilation_plan(&plan, configuration.target()) {
        report_mapped_diagnostic(source_map, input_path, "", module_diagnostic(&error));
        return 1;
    }
    if mode.is_strict() {
        println!("checked `{}` successfully (strict)", input_path.display());
    } else {
        println!("checked `{}` successfully", input_path.display());
    }
    0
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
