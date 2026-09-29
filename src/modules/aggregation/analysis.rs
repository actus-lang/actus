use std::collections::HashMap;
use std::path::PathBuf;

use crate::ast::Program;
use crate::semantic::{SemanticModel, analyze, filter_program_for_target};

use super::super::resolver::ModuleResolver;
use super::parsing::parse_module_with_overlays;
use super::plan::build_compilation_plan;
use super::types::ModuleError;
use super::unit::{ModuleUnit, load_module_unit};

pub fn analyze_module(
    resolver: &ModuleResolver,
    module_path: &str,
) -> Result<SemanticModel, ModuleError> {
    let unit = load_module_unit(resolver, module_path)?;
    analyze_module_unit(&unit)
}

pub fn analyze_module_unit(unit: &ModuleUnit) -> Result<SemanticModel, ModuleError> {
    analyze(unit.implementation()).map_err(|error| ModuleError::Semantic(Box::new(error)))
}

pub fn analyze_module_with_overlays(
    resolver: &ModuleResolver,
    module_path: &str,
    overlays: &HashMap<PathBuf, String>,
) -> Result<SemanticModel, ModuleError> {
    let program = parse_module_with_overlays(resolver, module_path, overlays)?;
    let program = resolve_imports(&program, resolver)?;
    analyze(&program).map_err(|error| ModuleError::Semantic(Box::new(error)))
}

pub fn analyze_module_with_overlays_for_target(
    resolver: &ModuleResolver,
    module_path: &str,
    overlays: &HashMap<PathBuf, String>,
    target: &crate::target::TargetSpec,
) -> Result<SemanticModel, ModuleError> {
    let program = parse_module_with_overlays(resolver, module_path, overlays)?;
    let program = resolve_imports(&program, resolver)?;
    let program = filter_program_for_target(&program, target);
    analyze(&program).map_err(|error| ModuleError::Semantic(Box::new(error)))
}

pub fn analyze_with_imports(
    program: &Program,
    resolver: &ModuleResolver,
) -> Result<SemanticModel, ModuleError> {
    let expanded = resolve_imports(program, resolver)?;
    analyze(&expanded).map_err(|error| ModuleError::Semantic(Box::new(error)))
}

pub fn analyze_with_imports_for_target(
    program: &Program,
    resolver: &ModuleResolver,
    target: &crate::target::TargetSpec,
) -> Result<SemanticModel, ModuleError> {
    let expanded = resolve_imports(program, resolver)?;
    let filtered = filter_program_for_target(&expanded, target);
    analyze(&filtered).map_err(|error| ModuleError::Semantic(Box::new(error)))
}

pub fn analyze_with_imports_with_overlays_for_target(
    program: &Program,
    resolver: &ModuleResolver,
    overlays: &HashMap<PathBuf, String>,
    target: &crate::target::TargetSpec,
) -> Result<SemanticModel, ModuleError> {
    let expanded = resolve_imports_with_overlays(program, resolver, overlays)?;
    let filtered = filter_program_for_target(&expanded, target);
    analyze(&filtered).map_err(|error| ModuleError::Semantic(Box::new(error)))
}

pub fn resolve_imports(
    program: &Program,
    resolver: &ModuleResolver,
) -> Result<Program, ModuleError> {
    Ok(build_compilation_plan(program, resolver)?.caller().clone())
}

fn resolve_imports_with_overlays(
    program: &Program,
    resolver: &ModuleResolver,
    overlays: &HashMap<PathBuf, String>,
) -> Result<Program, ModuleError> {
    Ok(super::plan::build_compilation_plan_with_overlays(program, resolver, overlays)?
        .caller()
        .clone())
}
