use std::collections::HashMap;
use std::path::PathBuf;

use crate::ast::{ExternalVerbDecl, ForeignAbi, Program, TopLevelDecl};
use crate::semantic::{SemanticModel, analyze, filter_program_for_target};

use super::super::resolver::ModuleResolver;
use super::parsing::parse_module_with_overlays;
use super::plan::{build_compilation_plan, build_module_compilation_plan_with_overlays};
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
    let program = resolve_module_imports_with_overlays(&program, resolver, overlays, module_path)?;
    analyze(&program).map_err(|error| ModuleError::Semantic(Box::new(error)))
}

pub fn analyze_module_with_overlays_for_target(
    resolver: &ModuleResolver,
    module_path: &str,
    overlays: &HashMap<PathBuf, String>,
    target: &crate::target::TargetSpec,
) -> Result<SemanticModel, ModuleError> {
    let program = parse_module_with_overlays(resolver, module_path, overlays)?;
    let program = resolve_module_imports_with_overlays(&program, resolver, overlays, module_path)?;
    let program = filter_program_for_target(&program, target);
    analyze(&program).map_err(|error| ModuleError::Semantic(Box::new(error)))
}

pub fn analyze_with_imports(
    program: &Program,
    resolver: &ModuleResolver,
) -> Result<SemanticModel, ModuleError> {
    let plan = build_compilation_plan(program, resolver)?;
    analyze_imported_units(&plan, resolver, &HashMap::new())?;
    let expanded = plan.caller().clone();
    analyze(&expanded).map_err(|error| ModuleError::Semantic(Box::new(error)))
}

pub fn analyze_with_imports_for_target(
    program: &Program,
    resolver: &ModuleResolver,
    target: &crate::target::TargetSpec,
) -> Result<SemanticModel, ModuleError> {
    let plan = build_compilation_plan(program, resolver)?;
    analyze_imported_units(&plan, resolver, &HashMap::new())?;
    let expanded = plan.caller().clone();
    let filtered = filter_program_for_target(&expanded, target);
    analyze(&filtered).map_err(|error| ModuleError::Semantic(Box::new(error)))
}

pub fn analyze_with_imports_with_overlays_for_target(
    program: &Program,
    resolver: &ModuleResolver,
    overlays: &HashMap<PathBuf, String>,
    target: &crate::target::TargetSpec,
) -> Result<SemanticModel, ModuleError> {
    let plan = super::plan::build_compilation_plan_with_overlays(program, resolver, overlays)?;
    analyze_imported_units(&plan, resolver, overlays)?;
    let expanded = plan.caller().clone();
    let filtered = filter_program_for_target(&expanded, target);
    analyze(&filtered).map_err(|error| ModuleError::Semantic(Box::new(error)))
}

fn analyze_imported_units(
    plan: &super::plan::ModuleCompilationPlan,
    resolver: &ModuleResolver,
    overlays: &HashMap<PathBuf, String>,
) -> Result<(), ModuleError> {
    for unit in plan.units() {
        let program =
            parse_module_with_overlays(resolver, unit.identity().module_path(), overlays)?;
        let module_plan = build_module_compilation_plan_with_overlays(
            unit.identity().module_path(),
            &program,
            resolver,
            overlays,
        )?;
        let expanded = module_scope_program(module_plan.caller());
        analyze(&expanded).map_err(|error| ModuleError::Semantic(Box::new(error)))?;
    }
    Ok(())
}

fn module_scope_program(program: &Program) -> Program {
    let declarations = program
        .declarations
        .iter()
        .map(|declaration| match declaration {
            TopLevelDecl::Verb(verb) => TopLevelDecl::ExternalVerb(ExternalVerbDecl {
                is_open: verb.is_open,
                doc: verb.doc.clone(),
                contract: verb.contract.clone(),
                unsafe_boundary: false,
                module_import: true,
                abi: ForeignAbi::C,
                metadata: verb.metadata.clone(),
                name: verb.name.clone(),
                native_symbol: None,
                generic_parameters: verb.generic_parameters.clone(),
                params: verb.params.clone(),
                return_type: verb.return_type.clone(),
                span: verb.span,
            }),
            _ => declaration.clone(),
        })
        .collect();
    Program { file_metadata: program.file_metadata.clone(), declarations }
}

pub fn resolve_imports(
    program: &Program,
    resolver: &ModuleResolver,
) -> Result<Program, ModuleError> {
    Ok(build_compilation_plan(program, resolver)?.caller().clone())
}

fn resolve_module_imports_with_overlays(
    program: &Program,
    resolver: &ModuleResolver,
    overlays: &HashMap<PathBuf, String>,
    module_path: &str,
) -> Result<Program, ModuleError> {
    Ok(build_module_compilation_plan_with_overlays(module_path, program, resolver, overlays)?
        .caller()
        .clone())
}
