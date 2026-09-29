use std::collections::HashSet;

use crate::ast::{Program, TopLevelDecl};

use super::super::resolver::ModuleResolver;
use super::types::ModuleError;
use super::unit::{ModuleUnit, load_module_unit};
use super::validation::export_identity;
use super::visibility::validate_import_visibility;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleCompilationPlan {
    caller: Program,
    units: Vec<ModuleUnit>,
}

impl ModuleCompilationPlan {
    pub fn caller(&self) -> &Program {
        &self.caller
    }

    pub fn units(&self) -> &[ModuleUnit] {
        &self.units
    }
}

pub fn build_compilation_plan(
    program: &Program,
    resolver: &ModuleResolver,
) -> Result<ModuleCompilationPlan, ModuleError> {
    let mut caller_declarations = Vec::new();
    let mut units = Vec::new();
    let mut imported_paths = HashSet::new();
    for declaration in &program.declarations {
        let TopLevelDecl::Import(import) = declaration else {
            caller_declarations.push(declaration.clone());
            continue;
        };
        if !imported_paths.insert(import.path.clone()) {
            continue;
        }
        let unit = load_module_unit(resolver, &import.path)?;
        validate_import_visibility(program, &import.path, &unit)?;
        append_public_declarations(&mut caller_declarations, &unit);
        units.push(unit);
    }
    Ok(ModuleCompilationPlan { caller: Program { declarations: caller_declarations }, units })
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
