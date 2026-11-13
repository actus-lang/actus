use std::fs;
use std::path::Path;

use crate::SharedSourceMap;
use crate::configuration::CompilerConfiguration;
use crate::diagnostics::{lex_diagnostic, module_diagnostic, parse_diagnostic};
use crate::modules::{
    ModuleCompilationPlan, ModuleResolver, build_compilation_plan, validate_compilation_plan,
};
use crate::parser::parse;

use super::super::diagnostics::{report_mapped_diagnostic, report_mapped_diagnostics};

pub(super) fn load_build_program(
    input: &str,
    configuration: &CompilerConfiguration,
) -> Option<(String, ModuleCompilationPlan, SharedSourceMap)> {
    let source_map = SharedSourceMap::new();
    let source = read_build_source(input)?;
    let program = parse_build_source(input, &source_map, &source)?;
    let program =
        resolve_build_program(input, &source, program, configuration, source_map.clone())?;
    Some((source, program, source_map))
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

fn parse_build_source(
    input: &str,
    source_map: &SharedSourceMap,
    source: &str,
) -> Option<crate::ast::Program> {
    let (tokens, lex_errors) = source_map.scan(input, source);
    if lex_errors.is_empty() {
        match parse(tokens) {
            Ok(program) => Some(program),
            Err(error) => {
                report_mapped_diagnostic(
                    source_map,
                    Path::new(input),
                    source,
                    parse_diagnostic(&error),
                );
                None
            }
        }
    } else {
        report_mapped_diagnostics(
            source_map,
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
    source_map: SharedSourceMap,
) -> Option<ModuleCompilationPlan> {
    let resolver = ModuleResolver::with_dependencies_and_runtime(
        configuration.source_root(),
        configuration.dependency_roots(),
        configuration.runtime_source_root(),
        configuration.runtime_module_roots(),
    )
    .with_source_map(source_map.clone());
    match build_compilation_plan(&program, &resolver) {
        Ok(plan) => Some(plan),
        Err(error) => {
            let diagnostic = module_diagnostic(&error);
            report_mapped_diagnostic(&source_map, Path::new(input), source, diagnostic);
            None
        }
    }
}

pub(super) fn validate_plan(
    input: &str,
    source: &str,
    plan: &ModuleCompilationPlan,
    target: &crate::target::TargetSpec,
    source_map: &SharedSourceMap,
) -> bool {
    if let Err(error) = validate_compilation_plan(plan, target) {
        report_mapped_diagnostic(source_map, Path::new(input), source, module_diagnostic(&error));
        return false;
    }
    true
}
