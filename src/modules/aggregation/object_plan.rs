use std::collections::HashSet;

use crate::ast::{ExternalVerbDecl, ForeignAbi, Program, TopLevelDecl};

use super::identity::ModuleNamespace;
use super::plan::ModuleCompilationPlan;
use super::types::ModuleError;
use super::validation::export_identity;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModuleObjectOwner {
    Root,
    Imported { module_path: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleObjectUnit {
    owner: ModuleObjectOwner,
    namespace: ModuleNamespace,
    program: Program,
    exported_verbs: Vec<String>,
}

impl ModuleObjectUnit {
    pub fn owner(&self) -> &ModuleObjectOwner {
        &self.owner
    }

    pub fn namespace(&self) -> &ModuleNamespace {
        &self.namespace
    }

    pub fn program(&self) -> &Program {
        &self.program
    }

    pub fn exported_verbs(&self) -> &[String] {
        &self.exported_verbs
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleObjectPlan {
    units: Vec<ModuleObjectUnit>,
}

impl ModuleObjectPlan {
    pub fn units(&self) -> &[ModuleObjectUnit] {
        &self.units
    }
}

pub(super) fn build_object_plan(
    compilation: &ModuleCompilationPlan,
) -> Result<ModuleObjectPlan, ModuleError> {
    let mut owners = HashSet::new();
    owners.insert(String::new());
    let mut units = vec![ModuleObjectUnit {
        owner: ModuleObjectOwner::Root,
        namespace: ModuleNamespace::root(),
        program: compilation.caller().clone(),
        exported_verbs: Vec::new(),
    }];
    let mut imported = compilation.units().iter().collect::<Vec<_>>();
    imported
        .sort_by(|left, right| left.identity().module_path().cmp(right.identity().module_path()));
    for module in imported {
        let module_path = module.identity().module_path().to_owned();
        if !owners.insert(module_path.clone()) {
            return Err(ModuleError::DuplicateObjectOwner { module_path });
        }
        units.push(ModuleObjectUnit {
            owner: ModuleObjectOwner::Imported { module_path },
            namespace: module.identity().namespace().clone(),
            program: implementation_declarations(module, compilation),
            exported_verbs: exported_implementation_names(module),
        });
    }
    Ok(ModuleObjectPlan { units })
}

fn exported_implementation_names(module: &super::unit::ModuleUnit) -> Vec<String> {
    module
        .implementation()
        .declarations
        .iter()
        .filter_map(|declaration| {
            let TopLevelDecl::Verb(verb) = declaration else { return None };
            module.exports().contains("verb", &verb.name).then(|| verb.name.clone())
        })
        .collect()
}

fn implementation_declarations(
    module: &super::unit::ModuleUnit,
    compilation: &ModuleCompilationPlan,
) -> Program {
    let local_declarations = module
        .implementation()
        .declarations
        .iter()
        .filter(|declaration| {
            !matches!(declaration, TopLevelDecl::Import(_) | TopLevelDecl::OpenSibling(_))
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut declarations = Vec::new();
    let mut imported_paths = HashSet::new();
    let imports = module
        .implementation()
        .declarations
        .iter()
        .filter_map(|declaration| {
            let TopLevelDecl::Import(import) = declaration else { return None };
            imported_paths.insert(import.path.clone()).then(|| {
                compilation.units().iter().find(|unit| unit.identity().module_path() == import.path)
            })?
        })
        .collect::<Vec<_>>();
    for dependency in imports {
        declarations.extend(
            dependency
                .implementation()
                .declarations
                .iter()
                .filter_map(|declaration| exported_dependency_declaration(dependency, declaration)),
        );
    }
    declarations.extend(local_declarations);
    append_root_generic_support(&mut declarations, module, compilation);
    Program { file_metadata: Vec::new(), declarations }
}

fn append_root_generic_support(
    declarations: &mut Vec<TopLevelDecl>,
    module: &super::unit::ModuleUnit,
    compilation: &ModuleCompilationPlan,
) {
    let existing = declarations.iter().filter_map(export_identity).collect::<HashSet<_>>();
    for declaration in &compilation.local().declarations {
        if matches!(declaration, TopLevelDecl::Import(_) | TopLevelDecl::OpenSibling(_)) {
            continue;
        }
        let Some(identity) = export_identity(declaration) else { continue };
        if existing.contains(&identity) || matches!(declaration, TopLevelDecl::Verb(_)) {
            continue;
        }
        if module.identity().module_path() != "<root>" {
            declarations.push(declaration.clone());
        }
    }
}

fn exported_dependency_declaration(
    dependency: &super::unit::ModuleUnit,
    declaration: &TopLevelDecl,
) -> Option<TopLevelDecl> {
    let (kind, name) = export_identity(declaration)?;
    dependency.exports().contains(kind, &name).then(|| dependency_declaration(declaration))
}

fn dependency_declaration(declaration: &TopLevelDecl) -> TopLevelDecl {
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
        TopLevelDecl::ExternalVerb(verb) => {
            let mut imported = verb.clone();
            imported.unsafe_boundary = false;
            imported.module_import = true;
            TopLevelDecl::ExternalVerb(imported)
        }
        _ => declaration.clone(),
    }
}
