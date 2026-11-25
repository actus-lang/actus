use std::fs;
use std::path::Path;

use crate::SharedSourceMap;
use crate::configuration::CompilerConfiguration;
use crate::diagnostics::module_diagnostic;
use crate::modules::{
    ModuleCompilationPlan, ModuleResolver, build_compilation_plan, validate_compilation_plan,
};
use crate::session::CompilationSession;

use super::super::diagnostics::{report_frontend_error, report_mapped_diagnostic};

pub(super) fn load_build_program(
    input: &str,
    configuration: &CompilerConfiguration,
) -> Option<(String, ModuleCompilationPlan, SharedSourceMap)> {
    let session = CompilationSession::new();
    let source = read_build_source(input)?;
    let program = parse_build_source(input, &session, &source)?;
    let program = resolve_build_program(input, &source, program, configuration, &session)?;
    Some((source, program, session.source_map().clone()))
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
    session: &CompilationSession,
    source: &str,
) -> Option<crate::ast::Program> {
    match session.parse(input, source).program() {
        Ok(program) => Some(program),
        Err(error) => {
            report_frontend_error(session.source_map(), Path::new(input), source, &error);
            None
        }
    }
}

fn resolve_build_program(
    input: &str,
    source: &str,
    program: crate::ast::Program,
    configuration: &CompilerConfiguration,
    session: &CompilationSession,
) -> Option<ModuleCompilationPlan> {
    let resolver = ModuleResolver::with_dependencies_and_runtime(
        configuration.source_root(),
        configuration.dependency_roots(),
        configuration.runtime_source_root(),
        configuration.runtime_module_roots(),
    )
    .with_package_identity(configuration.package_identity(), configuration.source_root())
    .with_session(session.clone());
    let plan =
        crate::cli::input::reject_production_tests(configuration, Path::new(input), &program)
            .and_then(|()| build_compilation_plan(&program, &resolver));
    match plan {
        Ok(plan) => Some(plan),
        Err(error) => {
            let diagnostic = module_diagnostic(&error);
            report_mapped_diagnostic(session.source_map(), Path::new(input), source, diagnostic);
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
