use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::ast::{Program, TopLevelDecl};
use crate::semantic::{SemanticModel, analyze, filter_program_for_target};

use super::super::resolver::ModuleResolver;
use super::exports::exports_module;
use super::parsing::{parse_module, parse_module_with_overlays};
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

pub fn resolve_imports(
    program: &Program,
    resolver: &ModuleResolver,
) -> Result<Program, ModuleError> {
    let mut declarations = Vec::new();
    let mut imported_paths = HashSet::new();
    for declaration in &program.declarations {
        let TopLevelDecl::Import(import) = declaration else {
            declarations.push(declaration.clone());
            continue;
        };
        if !imported_paths.insert(import.path.clone()) {
            continue;
        }
        let imported = parse_module(resolver, &import.path)?;
        let exports = exports_module(resolver, &import.path)?;
        declarations.extend(imported.declarations.into_iter().filter(|candidate| {
            let Some((kind, name)) = super::validation::export_identity(candidate) else {
                return false;
            };
            exports.contains(kind, &name)
        }));
    }
    Ok(Program { declarations })
}
