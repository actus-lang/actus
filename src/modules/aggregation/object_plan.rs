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
        let internal_scope = module_is_reachable_from_friend_scope(compilation, module);
        let exported_verbs = exported_implementation_names(module, internal_scope);
        units.push(ModuleObjectUnit {
            owner: ModuleObjectOwner::Imported { module_path },
            namespace: module.identity().namespace().clone(),
            program: implementation_declarations(module, compilation, generic_types),
            exported_verbs,
        });
    }
    Ok(ModuleObjectPlan { units })
}

fn module_is_reachable_from_friend_scope(
    compilation: &ModuleCompilationPlan,
    module: &super::unit::ModuleUnit,
) -> bool {
    let target = module.identity().module_path();
    let mut pending = compilation.friend_modules().iter().cloned().collect::<Vec<_>>();
    let mut visited = HashSet::new();
    while let Some(current) = pending.pop() {
        if !visited.insert(current.clone()) {
            continue;
        }
        if current == target || target.starts_with(&format!("{current}::")) {
            return true;
        }
        let Some(unit) =
            compilation.units().iter().find(|unit| unit.identity().module_path() == current)
        else {
            continue;
        };
        for declaration in &unit.implementation().declarations {
            let crate::ast::TopLevelDecl::Import(import) = declaration else { continue };
            pending.push(super::plan::qualify_import_path(&current, &import.path));
        }
    }
    false
}

fn exported_implementation_names(
    module: &super::unit::ModuleUnit,
    internal_scope: bool,
) -> Vec<String> {
    module
        .implementation()
        .declarations
        .iter()
        .filter_map(|declaration| {
            let TopLevelDecl::Verb(verb) = declaration else { return None };
            (internal_scope || verb.is_open).then(|| verb.name.clone())
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
        .filter(|declaration| !matches!(declaration, TopLevelDecl::OpenSibling(_)))
        .cloned()
        .collect::<Vec<_>>();
    let mut declarations = local_declarations
        .iter()
        .filter(|declaration| matches!(declaration, TopLevelDecl::Import(_)))
        .cloned()
        .collect::<Vec<_>>();
    let mut imported_paths = HashSet::new();
    let mut declaration_identities = local_declarations
        .iter()
        .filter_map(export_identity)
        .map(|(kind, name)| (kind.to_owned(), name))
        .collect::<HashSet<_>>();
    append_imported_declarations(
        module,
        compilation,
        &mut imported_paths,
        &mut declaration_identities,
        &mut declarations,
    );
    declarations.extend(
        local_declarations
            .into_iter()
            .filter(|declaration| !matches!(declaration, TopLevelDecl::Import(_))),
    );
    append_friend_import_aliases(
        module,
        compilation,
        &mut declaration_identities,
        &mut declarations,
    );
    append_shared_internal_declarations(
        compilation,
        &mut declaration_identities,
        &mut declarations,
    );
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

fn append_friend_import_aliases(
    module: &super::unit::ModuleUnit,
    compilation: &ModuleCompilationPlan,
    declaration_identities: &mut HashSet<(String, String)>,
    declarations: &mut Vec<TopLevelDecl>,
) {
    let local = module
        .implementation()
        .declarations
        .iter()
        .filter(|declaration| {
            matches!(declaration, TopLevelDecl::Verb(_) | TopLevelDecl::ExternalVerb(_))
        })
        .collect::<Vec<_>>();
    for import in module.implementation().declarations.iter().filter_map(|declaration| {
        let TopLevelDecl::Import(import) = declaration else { return None };
        if import.is_open {
            return None;
        }
        let path = super::plan::qualify_import_path(module.identity().module_path(), &import.path);
        let friend = compilation
            .friend_modules()
            .iter()
            .any(|root| path == *root || path.starts_with(&format!("{root}::")));
        friend.then(|| {
            import.alias.clone().or_else(|| import.path.rsplit("::").next().map(str::to_owned))
        })
    }) {
        let Some(namespace) = import else { continue };
        for declaration in &local {
            let imported = dependency_declaration(declaration, Some(&namespace));
            let Some((kind, name)) = export_identity(&imported) else { continue };
            if declaration_identities.insert((kind.to_owned(), name)) {
                declarations.push(imported);
            }
        }
    }
}

fn append_shared_internal_declarations(
    compilation: &ModuleCompilationPlan,
    declaration_identities: &mut HashSet<(String, String)>,
    declarations: &mut Vec<TopLevelDecl>,
) {
    if compilation.friend_modules().is_empty() {
        return;
    }
    for declaration in &compilation.caller().declarations {
        let include = match declaration {
            TopLevelDecl::Constant(_)
            | TopLevelDecl::Struct(_)
            | TopLevelDecl::Enum(_)
            | TopLevelDecl::Pack(_)
            | TopLevelDecl::Role(_) => true,
            TopLevelDecl::ExternalVerb(verb) => verb.module_import,
            _ => false,
        };
        if !include {
            continue;
        }
        let Some((kind, name)) = export_identity(declaration) else { continue };
        let identity = (kind.to_owned(), name);
        if declaration_identities.insert(identity) {
            declarations.push(declaration.clone());
        }
    }
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
        TopLevelDecl::Role(definition) => Some(("role".to_owned(), definition.name.clone())),
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
    let imports = imported_dependencies(module, compilation);
    for (dependency, namespace, open_import, selected) in imports {
        let first_visit = imported_paths.insert(dependency.identity().module_path().to_owned());
        append_dependency_imports(dependency, declarations);
        let module_path = module.identity().module_path();
        let dependency_path = dependency.identity().module_path();
        let parent_scope = module_path == dependency_path
            || module_path.starts_with(&format!("{dependency_path}::"))
            || dependency_path.starts_with(&format!("{module_path}::"));
        let friend = selected.is_empty()
            && !open_import
            && (parent_scope
                || compilation.friend_modules().iter().any(|friend| {
                    dependency_path == friend || dependency_path.starts_with(&format!("{friend}::"))
                }));
        for declaration in &dependency.implementation().declarations {
            if !selected.is_empty()
                && export_identity(declaration)
                    .is_none_or(|(_, name)| !selected.iter().any(|item| item == &name))
            {
                continue;
            }
            let qualified_namespace = if open_import {
                namespace.as_deref()
            } else {
                friend.then_some(namespace.as_deref()).flatten()
            };
            let Some(exported) = exported_dependency_declaration(
                dependency,
                declaration,
                friend,
                qualified_namespace,
            ) else {
                continue;
            };
            let Some((kind, name)) = export_identity(&exported) else { continue };
            let identity = (kind.to_owned(), name);
            if declaration_identities.insert(identity) {
                declarations.push(exported);
            }
            if open_import || !selected.is_empty() {
                let Some(opened) =
                    exported_dependency_declaration(dependency, declaration, false, None)
                else {
                    continue;
                };
                let Some((kind, name)) = export_identity(&opened) else { continue };
                let identity = (kind.to_owned(), name);
                if declaration_identities.insert(identity) {
                    declarations.push(opened);
                }
            }
        }
        if first_visit {
            append_imported_declarations(
                dependency,
                compilation,
                imported_paths,
                declaration_identities,
                declarations,
            );
        }
    }
}

fn append_dependency_imports(
    dependency: &super::unit::ModuleUnit,
    declarations: &mut Vec<TopLevelDecl>,
) {
    for declaration in &dependency.implementation().declarations {
        let TopLevelDecl::Import(import) = declaration else { continue };
        if import.is_open {
            continue;
        }
        let mut imported = import.clone();
        imported.path =
            super::plan::qualify_import_path(dependency.identity().module_path(), &import.path);
        let already_present = declarations.iter().any(|declaration| {
            let TopLevelDecl::Import(existing) = declaration else { return false };
            import_binding(existing) == import_binding(&imported)
                && existing.is_open == imported.is_open
        });
        if !already_present {
            declarations.push(TopLevelDecl::Import(imported));
        }
    }
}

fn imported_dependencies<'a>(
    module: &super::unit::ModuleUnit,
    compilation: &'a ModuleCompilationPlan,
) -> Vec<(&'a super::unit::ModuleUnit, Option<String>, bool, Vec<String>)> {
    module
        .implementation()
        .declarations
        .iter()
        .filter_map(|declaration| {
            let TopLevelDecl::Import(import) = declaration else { return None };
            let import_path =
                super::plan::qualify_import_path(module.identity().module_path(), &import.path);
            let unit = compilation
                .units()
                .iter()
                .find(|unit| unit.identity().module_path() == import_path)?;
            let namespace = Some({
                import.alias.clone().or_else(|| import.path.rsplit("::").next().map(str::to_owned))
            });
            Some((unit, namespace.flatten(), import.is_open, import.selected.clone()))
        })
        .collect()
}

fn import_binding(import: &crate::ast::ImportDecl) -> String {
    import
        .alias
        .clone()
        .or_else(|| import.path.rsplit("::").next().map(str::to_owned))
        .unwrap_or_else(|| import.path.clone())
}

fn exported_dependency_declaration(
    dependency: &super::unit::ModuleUnit,
    declaration: &TopLevelDecl,
    friend: bool,
    namespace: Option<&str>,
) -> Option<TopLevelDecl> {
    let (kind, name) = export_identity(declaration)?;
    (friend || dependency.exports().contains(kind, &name))
        .then(|| dependency_declaration(declaration, namespace))
}

fn dependency_declaration(declaration: &TopLevelDecl, namespace: Option<&str>) -> TopLevelDecl {
    match declaration {
        TopLevelDecl::Verb(verb) => TopLevelDecl::ExternalVerb(ExternalVerbDecl {
            is_open: verb.is_open,
            doc: verb.doc.clone(),
            contract: verb.contract.clone(),
            unsafe_boundary: false,
            module_import: true,
            abi: ForeignAbi::C,
            metadata: verb.metadata.clone(),
            name: namespace
                .map_or_else(|| verb.name.clone(), |value| format!("{value}::{}", verb.name)),
            native_symbol: namespace.map(|_| verb.name.clone()),
            generic_parameters: verb.generic_parameters.clone(),
            params: verb.params.clone(),
            return_type: verb.return_type.clone(),
            span: verb.span,
        }),
        TopLevelDecl::ExternalVerb(verb) => {
            let mut imported = verb.clone();
            imported.unsafe_boundary = false;
            imported.module_import = true;
            if let Some(namespace) = namespace {
                imported.native_symbol = Some(imported.name.clone());
                imported.name = format!("{namespace}::{}", imported.name);
            }
            TopLevelDecl::ExternalVerb(imported)
        }
        _ => declaration.clone(),
    }
}
