use std::fs;
use std::path::Path;

use crate::configuration::CompilerConfiguration;
use crate::diagnostics::{lex_diagnostic, module_diagnostic, parse_diagnostic, render_diagnostic};
use crate::lexer::scan;
use crate::modules::{ModuleCompilationPlan, ModuleResolver, build_compilation_plan};
use crate::parser::parse;

use super::super::diagnostics::{report_diagnostic, report_diagnostics};

pub(super) fn load_build_program(
    input: &str,
    configuration: &CompilerConfiguration,
) -> Option<(String, ModuleCompilationPlan)> {
    let source = read_build_source(input)?;
    let program = parse_build_source(input, &source)?;
    let program = resolve_build_program(input, &source, program, configuration)?;
    Some((source, program))
}

fn read_build_source(input: &str) -> Option<String> {
    match fs::read_to_string(input) {
        Ok(source) => Some(source),
        Err(error) => {
            eprintln!("error: cannot read `{input}`: {error}");
            None
        }
    }
}

fn parse_build_source(input: &str, source: &str) -> Option<crate::ast::Program> {
    let (tokens, lex_errors) = scan(source);
    if lex_errors.is_empty() {
        match parse(tokens) {
            Ok(program) => Some(program),
            Err(error) => {
                report_diagnostic(Path::new(input), source, parse_diagnostic(&error));
                None
            }
        }
    } else {
        report_diagnostics(
            Path::new(input),
            source,
            lex_errors.iter().map(lex_diagnostic).collect(),
        );
        None
    }
}

fn resolve_build_program(
    input: &str,
    source: &str,
    program: crate::ast::Program,
    configuration: &CompilerConfiguration,
) -> Option<ModuleCompilationPlan> {
    let resolver = ModuleResolver::with_dependencies_and_runtime(
        configuration.source_root(),
        configuration.dependency_roots(),
        configuration.runtime_source_root(),
        configuration.runtime_module_roots(),
    );
    match build_compilation_plan(&program, &resolver) {
        Ok(plan) => Some(plan),
        Err(error) => {
            let diagnostic = module_diagnostic(&error);
            eprintln!("{input}: {}", render_diagnostic(source, &diagnostic));
            None
        }
    }
}

pub(super) fn validate_strict_program(
    input: &str,
    source: &str,
    program: &crate::ast::Program,
) -> bool {
    match crate::semantic::analyze(program) {
        Ok(_) => true,
        Err(error) => {
            report_diagnostic(
                Path::new(input),
                source,
                crate::diagnostics::semantic_diagnostic(&error),
            );
            false
        }
    }
}

pub(super) fn validate_strict_plan(
    input: &str,
    source: &str,
    plan: &ModuleCompilationPlan,
) -> bool {
    if !validate_strict_program(input, source, plan.caller()) {
        return false;
    }
    let object_plan = match plan.object_plan() {
        Ok(object_plan) => object_plan,
        Err(error) => {
            eprintln!("{input}: {}", render_diagnostic(source, &module_diagnostic(&error)));
            return false;
        }
    };
    for unit in object_plan.units() {
        if let Err(error) = crate::semantic::analyze(unit.program()) {
            report_diagnostic(
                Path::new(input),
                source,
                crate::diagnostics::semantic_diagnostic(&error),
            );
            return false;
        }
    }
    true
}
