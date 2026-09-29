use std::collections::{HashMap, HashSet};

use crate::ast::{ExternalVerbDecl, ForeignAbi, Program, TopLevelDecl};

use super::super::resolver::ModuleResolver;
use super::object_plan::{ModuleObjectPlan, build_object_plan};
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

    pub fn object_plan(&self) -> Result<ModuleObjectPlan, ModuleError> {
        build_object_plan(self)
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
    validate_symbol_collisions(&local_declarations, &units)?;
    Ok(ModuleCompilationPlan {
        local: Program { declarations: local_declarations },
        caller: Program { declarations: caller_declarations },
        units,
    })
}

fn validate_symbol_collisions(
    local_declarations: &[TopLevelDecl],
    units: &[ModuleUnit],
) -> Result<(), ModuleError> {
    let mut symbols = HashMap::new();
    record_symbols("<root>", local_declarations, &mut symbols)?;
    for unit in units {
        record_symbols(
            unit.identity().module_path(),
            &unit.implementation().declarations,
            &mut symbols,
        )?;
    }
    Ok(())
}

fn record_symbols(
    module: &str,
    declarations: &[TopLevelDecl],
    symbols: &mut HashMap<(String, String), String>,
) -> Result<(), ModuleError> {
    for declaration in declarations {
        let Some((kind, name)) = export_identity(declaration) else { continue };
        let key = (kind.to_owned(), name.clone());
        if let Some(first_module) = symbols.insert(key, module.to_owned())
            && first_module != module
        {
            return Err(ModuleError::SymbolCollision {
                symbol: format!("{kind} `{name}`"),
                first_module,
                second_module: module.to_owned(),
            });
        }
    }
    Ok(())
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
            .map(caller_declaration),
    );
}

fn caller_declaration(declaration: &TopLevelDecl) -> TopLevelDecl {
    match declaration {
        TopLevelDecl::Verb(verb) => TopLevelDecl::ExternalVerb(ExternalVerbDecl {
            is_open: verb.is_open,
            doc: verb.doc.clone(),
            unsafe_boundary: false,
            module_import: true,
            abi: ForeignAbi::C,
            metadata: verb.metadata.clone(),
            name: verb.name.clone(),
            generic_parameters: verb.generic_parameters.clone(),
            params: verb.params.clone(),
            return_type: verb.return_type.clone(),
            span: verb.span,
        }),
        _ => declaration.clone(),
    }
}
