use std::fs;
use std::path::Path;

use crate::SharedSourceMap;
use crate::configuration::CompilerConfiguration;
use crate::diagnostics::{lex_diagnostic, module_diagnostic, parse_diagnostic};
use crate::modules::{ModuleCompilationPlan, ModuleResolver, build_compilation_plan};
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

pub(super) fn validate_strict_program(
    input: &str,
    source: &str,
    source_map: &SharedSourceMap,
    program: &crate::ast::Program,
) -> bool {
    match crate::semantic::analyze(program) {
        Ok(_) => true,
        Err(error) => {
            report_mapped_diagnostic(
                source_map,
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
    target: &crate::target::TargetSpec,
    source_map: &SharedSourceMap,
) -> bool {
    let caller = match crate::semantic::filter_program_for_target_checked(plan.caller(), target) {
        Ok(program) => program,
        Err(error) => {
            report_mapped_diagnostic(
                source_map,
                Path::new(input),
                source,
                crate::diagnostics::semantic_diagnostic(&error),
            );
            return false;
        }
    };
    if !validate_strict_program(input, source, source_map, &caller) {
        return false;
    }
    let object_plan = match plan.object_plan() {
        Ok(object_plan) => object_plan,
        Err(error) => {
            report_mapped_diagnostic(
                source_map,
                Path::new(input),
                source,
                module_diagnostic(&error),
            );
            return false;
        }
    };
    for unit in object_plan.units() {
        let targeted =
            match crate::semantic::filter_program_for_target_checked(unit.program(), target) {
                Ok(program) => program,
                Err(error) => {
                    report_mapped_diagnostic(
                        source_map,
                        Path::new(input),
                        source,
                        crate::diagnostics::semantic_diagnostic(&error),
                    );
                    return false;
                }
            };
        if let Err(error) = crate::semantic::analyze(&targeted) {
            report_mapped_diagnostic(
                source_map,
                Path::new(input),
                source,
                crate::diagnostics::semantic_diagnostic(&error),
            );
            return false;
        }
    }
    true
}
