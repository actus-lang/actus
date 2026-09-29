use std::collections::HashSet;

use crate::ast::{Program, TopLevelDecl};

use super::super::resolver::ModuleResolver;
use super::types::ModuleError;
use super::unit::{ModuleUnit, load_module_unit};
use super::validation::export_identity;
use super::visibility::validate_import_visibility;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleCompilationPlan {
    local: Program,
    caller: Program,
    units: Vec<ModuleUnit>,
}

impl ModuleCompilationPlan {
    pub fn local(&self) -> &Program {
        &self.local
    }

    pub fn caller(&self) -> &Program {
        &self.caller
    }

    pub fn units(&self) -> &[ModuleUnit] {
        &self.units
    }

    pub fn implementation(&self) -> Program {
        let mut declarations = self.local.declarations.clone();
        for unit in &self.units {
            declarations.extend(
                unit.implementation()
                    .declarations
                    .iter()
                    .filter(|declaration| {
                        !matches!(
                            declaration,
                            TopLevelDecl::Import(_) | TopLevelDecl::OpenSibling(_)
                        )
                    })
                    .cloned(),
            );
        }
        Program { declarations }
    }
}

pub fn build_compilation_plan(
    program: &Program,
    resolver: &ModuleResolver,
) -> Result<ModuleCompilationPlan, ModuleError> {
    let mut caller_declarations = Vec::new();
    let mut local_declarations = Vec::new();
    let mut units = Vec::new();
    let mut imported_paths = HashSet::new();
    for declaration in &program.declarations {
        let TopLevelDecl::Import(import) = declaration else {
            local_declarations.push(declaration.clone());
            caller_declarations.push(declaration.clone());
            continue;
        };
        if !imported_paths.insert(import.path.clone()) {
            continue;
        }
        collect_unit(
            &import.path,
            program,
            resolver,
            &mut imported_paths,
            &mut units,
            &mut caller_declarations,
        )?;
    }
    Ok(ModuleCompilationPlan {
        local: Program { declarations: local_declarations },
        caller: Program { declarations: caller_declarations },
        units,
    })
}

fn collect_unit(
    module_path: &str,
    importer: &Program,
    resolver: &ModuleResolver,
    imported_paths: &mut HashSet<String>,
    units: &mut Vec<ModuleUnit>,
    caller_declarations: &mut Vec<TopLevelDecl>,
) -> Result<(), ModuleError> {
    let unit = load_module_unit(resolver, module_path)?;
    validate_import_visibility(importer, module_path, &unit)?;
    for declaration in &unit.implementation().declarations {
        let TopLevelDecl::Import(import) = declaration else { continue };
        if imported_paths.insert(import.path.clone()) {
            collect_unit(
                &import.path,
                unit.implementation(),
                resolver,
                imported_paths,
                units,
                caller_declarations,
            )?;
        }
    }
    append_public_declarations(caller_declarations, &unit);
    units.push(unit);
    Ok(())
}

fn append_public_declarations(target: &mut Vec<TopLevelDecl>, unit: &ModuleUnit) {
    target.extend(
        unit.implementation()
            .declarations
            .iter()
            .filter(|declaration| {
                let Some((kind, name)) = export_identity(declaration) else { return false };
                unit.exports().contains(kind, &name)
            })
            .cloned(),
    );
}
