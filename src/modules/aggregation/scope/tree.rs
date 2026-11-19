//! The directory-module tree of one loaded unit (ADR-0018, ADR-0110).

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{ImportDecl, TopLevelDecl};

use super::super::unit::{ModuleSource, ModuleSourceKind, ModuleUnit};
use super::super::validation::export_identity;

/// One directory module that is compiled as part of its unit.
#[derive(Debug, Default)]
pub(super) struct DirectoryModule {
    /// Sibling files and child directories named by the facade's `open`.
    opened: BTreeSet<String>,
    /// Declaration name to `(declaring file stem, declared open)`.
    declarations: BTreeMap<String, (String, bool)>,
    /// Imports written in any file of the directory module.
    pub(super) imports: Vec<ImportDecl>,
}

#[derive(Debug)]
pub(crate) struct ModuleTree {
    unit_path: String,
    modules: BTreeMap<String, DirectoryModule>,
    owners: BTreeMap<String, String>,
}

impl ModuleTree {
    pub(crate) fn of_unit(unit: &ModuleUnit) -> Self {
        Self::from_sources(unit.identity().module_path(), unit.sources())
    }

    pub(crate) fn from_sources(unit_path: &str, sources: &[ModuleSource]) -> Self {
        let mut modules = BTreeMap::<String, DirectoryModule>::new();
        for source in sources {
            let (path, stem) = source_location(unit_path, source);
            let module = modules.entry(path).or_default();
            record_source(module, unit_path, stem, source);
        }
        let members = member_paths(unit_path, &modules);
        modules.retain(|path, _| members.contains(path));
        let owners = modules
            .iter()
            .flat_map(|(path, module)| {
                module.declarations.keys().map(move |name| (name.clone(), path.clone()))
            })
            .collect();
        Self { unit_path: unit_path.to_owned(), modules, owners }
    }

    pub(super) fn unit_path(&self) -> &str {
        &self.unit_path
    }

    pub(crate) fn contains(&self, module_path: &str) -> bool {
        self.modules.contains_key(module_path)
    }

    pub(super) fn module(&self, module_path: &str) -> Option<&DirectoryModule> {
        self.modules.get(module_path)
    }

    pub(super) fn modules(&self) -> impl Iterator<Item = (&String, &DirectoryModule)> {
        self.modules.iter()
    }

    /// The directory module that declares `name` within this unit.
    pub(super) fn owner(&self, name: &str) -> Option<&str> {
        self.owners.get(name).map(String::as_str)
    }

    pub(super) fn declares(&self, module_path: &str, name: &str) -> bool {
        self.modules.get(module_path).is_some_and(|module| module.declarations.contains_key(name))
    }

    /// Whether `module_path` exports `name` through its facade, directly or
    /// through a child directory it opens (ADR-0018).
    pub(super) fn exports(&self, module_path: &str, name: &str) -> bool {
        let Some(module) = self.modules.get(module_path) else { return false };
        if let Some((file, is_open)) = module.declarations.get(name)
            && *is_open
            && module.opened.contains(file)
        {
            return true;
        }
        module.opened.iter().any(|opened| {
            let child = format!("{module_path}::{opened}");
            self.modules.contains_key(&child) && self.exports(&child, name)
        })
    }
}

pub(super) fn is_ancestor(ancestor: &str, module_path: &str) -> bool {
    module_path.len() > ancestor.len()
        && module_path.starts_with(ancestor)
        && module_path[ancestor.len()..].starts_with("::")
}

/// The child of `ancestor` on the path down to `descendant`.
pub(super) fn child_toward(ancestor: &str, descendant: &str) -> String {
    let rest = &descendant[ancestor.len() + 2..];
    let segment = rest.split("::").next().unwrap_or(rest);
    format!("{ancestor}::{segment}")
}

fn source_location(unit_path: &str, source: &ModuleSource) -> (String, String) {
    let stem = source.path().file_stem().and_then(|stem| stem.to_str()).unwrap_or_default();
    match source.kind() {
        ModuleSourceKind::Facade | ModuleSourceKind::Sibling { .. } => {
            (unit_path.to_owned(), stem.to_owned())
        }
        ModuleSourceKind::ChildFacade { module_path }
        | ModuleSourceKind::ChildSibling { module_path, .. } => {
            (module_path.clone(), stem.to_owned())
        }
    }
}

fn record_source(
    module: &mut DirectoryModule,
    unit_path: &str,
    stem: String,
    source: &ModuleSource,
) {
    let facade =
        matches!(source.kind(), ModuleSourceKind::Facade | ModuleSourceKind::ChildFacade { .. });
    for declaration in &source.program().declarations {
        match declaration {
            TopLevelDecl::OpenSibling(open) if facade => {
                module.opened.insert(open.name.clone());
            }
            TopLevelDecl::Import(import) => {
                let mut qualified = import.clone();
                qualified.path =
                    super::super::declarations::qualify_import_path(unit_path, &import.path);
                module.imports.push(qualified);
            }
            _ => {
                if let Some((kind, name)) = export_identity(declaration)
                    && kind != "perform"
                {
                    let is_open = super::super::exports::declaration_is_open(declaration);
                    module.declarations.entry(name).or_insert((stem.clone(), is_open));
                }
            }
        }
    }
}

/// Directory modules that belong to the unit: the unit root and every child
/// directory opened by its parent's facade.
fn member_paths(unit_path: &str, modules: &BTreeMap<String, DirectoryModule>) -> BTreeSet<String> {
    let mut members = BTreeSet::from([unit_path.to_owned()]);
    for path in modules.keys() {
        let Some((parent, name)) = path.rsplit_once("::") else { continue };
        if members.contains(parent)
            && modules.get(parent).is_some_and(|module| module.opened.contains(name))
        {
            members.insert(path.clone());
        }
    }
    members
}
