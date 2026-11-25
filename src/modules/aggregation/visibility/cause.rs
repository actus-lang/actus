use std::collections::HashSet;
use std::path::PathBuf;

use crate::ast::{Program, TopLevelDecl};
use crate::lexer::SourceSpan;

use super::super::exports::declaration_is_open;
use super::super::types::{ModuleError, PrivateCause};
use super::super::unit::{ModuleSource, ModuleSourceKind, ModuleUnit};
use super::super::validation::export_identity;

/// Builds the private-declaration error for `name`, naming why the facade
/// chain of `unit` does not export it.
pub(super) fn private_access(
    unit: &ModuleUnit,
    module_path: &str,
    name: &str,
    span: SourceSpan,
) -> ModuleError {
    ModuleError::PrivateDeclarationAccess {
        module: module_path.to_owned(),
        symbol: name.to_owned(),
        facade: unit.facade().to_owned(),
        span,
        cause: Box::new(cause(unit, name)),
    }
}

enum Opening {
    Public,
    UnitOnly(PathBuf),
    Missing,
}

/// The module a source belongs to and, for a non-facade file, its name.
fn membership<'a>(unit: &'a ModuleUnit, source: &'a ModuleSource) -> (&'a str, Option<&'a str>) {
    match source.kind() {
        ModuleSourceKind::Facade => (unit.identity().module_path(), None),
        ModuleSourceKind::Sibling { name } => (unit.identity().module_path(), Some(name)),
        ModuleSourceKind::ChildFacade { module_path } => (module_path, None),
        ModuleSourceKind::ChildSibling { module_path, name } => (module_path, Some(name)),
    }
}

fn cause(unit: &ModuleUnit, name: &str) -> PrivateCause {
    let declared = unit.sources().iter().find_map(|source| {
        source
            .program()
            .declarations
            .iter()
            .find(|declaration| {
                export_identity(declaration).is_some_and(|(kind, found)| {
                    found == name && !unit.exports().contains(kind, &found)
                })
            })
            .map(|declaration| (source, declaration))
    });
    let Some((source, declaration)) = declared else { return PrivateCause::NotOpen };
    let (module, Some(file)) = membership(unit, source) else { return PrivateCause::NotOpen };
    if !declaration_is_open(declaration) {
        return PrivateCause::NotOpen;
    }
    // Walk from the declaring file up to the unit root; the first facade that
    // does not open the next step publicly is the reason the name is hidden.
    let root = unit.identity().module_path();
    let (mut module, mut entry) = (module.to_owned(), file.to_owned());
    loop {
        match opening(unit, &module, &entry) {
            Opening::Public => {}
            Opening::UnitOnly(facade) => return PrivateCause::UnitOnly { entry, facade },
            Opening::Missing => {
                let facade = module_facade(unit, &module).unwrap_or_else(|| unit.facade());
                return PrivateCause::NotOpened { entry, facade: facade.to_owned() };
            }
        }
        let Some((parent, last)) = module.rsplit_once("::").filter(|_| module != root) else {
            return PrivateCause::NotOpen;
        };
        entry = last.to_owned();
        module = parent.to_owned();
    }
}

fn module_facade<'a>(unit: &'a ModuleUnit, module: &str) -> Option<&'a std::path::Path> {
    unit.sources()
        .iter()
        .find(|source| matches!(membership(unit, source), (found, None) if found == module))
        .map(ModuleSource::path)
}

/// How the facade of `module` (or a file it publicly opens) opens `entry`.
fn opening(unit: &ModuleUnit, module: &str, entry: &str) -> Opening {
    let members: Vec<(Option<&str>, &ModuleSource)> = unit
        .sources()
        .iter()
        .filter_map(|source| match membership(unit, source) {
            (found, file) if found == module => Some((file, source)),
            _ => None,
        })
        .collect();
    let mut reached: Vec<&ModuleSource> =
        members.iter().filter(|(file, _)| file.is_none()).map(|(_, source)| *source).collect();
    let mut seen = HashSet::new();
    let mut unit_only = None;
    let mut index = 0;
    while let Some(source) = reached.get(index).copied() {
        index += 1;
        for open in open_siblings(source.program()) {
            if open.unit && open.name == entry && unit_only.is_none() {
                unit_only = Some(source.path().to_owned());
            }
            if open.unit {
                continue;
            }
            if open.name == entry {
                return Opening::Public;
            }
            if seen.insert(open.name.as_str())
                && let Some((_, sibling)) =
                    members.iter().find(|(file, _)| *file == Some(&open.name))
            {
                reached.push(sibling);
            }
        }
    }
    unit_only.map_or(Opening::Missing, Opening::UnitOnly)
}

fn open_siblings(program: &Program) -> impl Iterator<Item = &crate::ast::OpenSiblingDecl> {
    program.declarations.iter().filter_map(|declaration| match declaration {
        TopLevelDecl::OpenSibling(open) => Some(open),
        _ => None,
    })
}
