use crate::diagnostics::{render_lex_error, render_parse_error, render_semantic_error};
use crate::lexer::scan;
use crate::modules::{ModuleError, ModuleResolver, parse_module, resolve_imports};
use crate::parser::parse;
use crate::semantic::filter_program_for_target;
use std::fs;

pub(super) fn check_command(mut arguments: impl Iterator<Item = String>) -> i32 {
    let explicit_input = arguments.next();
    if arguments.next().is_some() {
        eprintln!("error: unexpected extra argument");
        return 2;
    }
    let configuration = match super::input::configuration_for_input(explicit_input.as_deref()) {
        Ok(configuration) => configuration,
        Err(error) => {
            eprintln!("error: {error}");
            return 1;
        }
    };
    let input_path = super::input::entry_path(&configuration, explicit_input.as_deref());
    check_input(&configuration, &input_path, explicit_input.is_some())
}

fn check_input(
    configuration: &crate::configuration::CompilerConfiguration,
    input_path: &std::path::Path,
    explicit: bool,
) -> i32 {
    if explicit
        && let Some(module_path) =
            super::input::module_path_for_file(configuration.source_root(), input_path)
    {
        return check_module(configuration, input_path, &module_path);
    }
    check_source(configuration, input_path)
}

fn check_source(
    configuration: &crate::configuration::CompilerConfiguration,
    input_path: &std::path::Path,
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
    match crate::semantic::analyze(&filter_program_for_target(&program, configuration.target())) {
        Ok(_) => {
            println!("checked `{input}` successfully");
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
    match crate::semantic::analyze(&filter_program_for_target(&program, configuration.target())) {
        Ok(_) => {
            println!("checked `{}` successfully", input_path.display());
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
