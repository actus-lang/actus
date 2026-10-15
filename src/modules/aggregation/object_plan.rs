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
    generic_types: &[crate::ast::TypeName],
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
            program: implementation_declarations(module, compilation, generic_types),
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
            verb.is_open.then(|| verb.name.clone())
        })
        .collect()
}

fn implementation_declarations(
    module: &super::unit::ModuleUnit,
    compilation: &ModuleCompilationPlan,
    generic_types: &[crate::ast::TypeName],
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
    let mut declaration_identities = HashSet::new();
    append_imported_declarations(
        module,
        compilation,
        &mut imported_paths,
        &mut declaration_identities,
        &mut declarations,
    );
    declarations.extend(local_declarations);
    for declaration in &declarations {
        if let Some(identity) = type_declaration_identity(declaration) {
            declaration_identities.insert(identity);
        }
    }
    append_generic_type_declarations(
        compilation,
        generic_types,
        &mut declaration_identities,
        &mut declarations,
    );
    Program { file_metadata: Vec::new(), declarations }
}

fn append_generic_type_declarations(
    compilation: &ModuleCompilationPlan,
    generic_types: &[crate::ast::TypeName],
    declaration_identities: &mut HashSet<(String, String)>,
    declarations: &mut Vec<TopLevelDecl>,
) {
    for type_name in generic_types {
        append_generic_type_declaration(
            compilation,
            type_name,
            declaration_identities,
            declarations,
        );
    }
}

fn append_generic_type_declaration(
    compilation: &ModuleCompilationPlan,
    type_name: &crate::ast::TypeName,
    declaration_identities: &mut HashSet<(String, String)>,
    declarations: &mut Vec<TopLevelDecl>,
) {
    for argument in &type_name.arguments {
        append_generic_type_declaration(
            compilation,
            argument,
            declaration_identities,
            declarations,
        );
    }
    let Some(declaration) =
        compilation.caller().declarations.iter().find(|declaration| match declaration {
            TopLevelDecl::Struct(definition) => definition.name == type_name.name,
            TopLevelDecl::Enum(definition) => definition.name == type_name.name,
            TopLevelDecl::Pack(definition) => definition.name == type_name.name,
            _ => false,
        })
    else {
        return;
    };
    if let Some(identity) = type_declaration_identity(declaration)
        && declaration_identities.insert(identity)
    {
        let declaration = declaration.clone();
        declarations.push(declaration.clone());
        for dependency in declaration_type_dependencies(&declaration) {
            append_generic_type_declaration(
                compilation,
                dependency,
                declaration_identities,
                declarations,
            );
        }
    }
}

fn declaration_type_dependencies(declaration: &TopLevelDecl) -> Vec<&crate::ast::TypeName> {
    match declaration {
        TopLevelDecl::Struct(definition) => {
            definition.fields.iter().map(|field| &field.ty).collect()
        }
        TopLevelDecl::Enum(definition) => definition
            .variants
            .iter()
            .flat_map(|variant| match &variant.payload {
                crate::ast::EnumPayload::Unit => Vec::new(),
                crate::ast::EnumPayload::Tuple(fields) => fields.iter().collect(),
                crate::ast::EnumPayload::Struct(fields) => {
                    fields.iter().map(|field| &field.ty).collect()
                }
            })
            .collect(),
        TopLevelDecl::Pack(definition) => {
            let mut dependencies = vec![definition.storage.type_name()];
            dependencies.extend(definition.fields.iter().map(|field| &field.ty));
            dependencies
        }
        _ => Vec::new(),
    }
}

fn type_declaration_identity(declaration: &TopLevelDecl) -> Option<(String, String)> {
    match declaration {
        TopLevelDecl::Struct(definition) => Some(("struct".to_owned(), definition.name.clone())),
        TopLevelDecl::Enum(definition) => Some(("enum".to_owned(), definition.name.clone())),
        TopLevelDecl::Pack(definition) => Some(("pack".to_owned(), definition.name.clone())),
        _ => None,
    }
}

fn append_imported_declarations(
    module: &super::unit::ModuleUnit,
    compilation: &ModuleCompilationPlan,
    imported_paths: &mut HashSet<String>,
    declaration_identities: &mut HashSet<(String, String)>,
    declarations: &mut Vec<TopLevelDecl>,
) {
    let imports = module
        .implementation()
        .declarations
        .iter()
        .filter_map(|declaration| {
            let TopLevelDecl::Import(import) = declaration else { return None };
            let import_path =
                super::plan::qualify_import_path(module.identity().module_path(), &import.path);
            compilation.units().iter().find(|unit| unit.identity().module_path() == import_path)
        })
        .collect::<Vec<_>>();
    for dependency in imports {
        if !imported_paths.insert(dependency.identity().module_path().to_owned()) {
            continue;
        }
        for declaration in &dependency.implementation().declarations {
            let Some(exported) = exported_dependency_declaration(dependency, declaration) else {
                continue;
            };
            let Some((kind, name)) = export_identity(&exported) else { continue };
            let identity = (kind.to_owned(), name);
            if declaration_identities.insert(identity) {
                declarations.push(exported);
            }
        }
        append_imported_declarations(
            dependency,
            compilation,
            imported_paths,
            declaration_identities,
            declarations,
        );
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
        TopLevelDecl::ExternalVerb(verb) => {
            let mut imported = verb.clone();
            imported.unsafe_boundary = false;
            imported.module_import = true;
            TopLevelDecl::ExternalVerb(imported)
        }
        _ => declaration.clone(),
    }
}
