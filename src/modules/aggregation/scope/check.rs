//! Hierarchical module scope validation (ADR-0110).
//!
//! An unqualified spelling in directory module `M` names a declaration of
//! its unit only when that declaration is declared by `M` or an ancestor of
//! `M`, exported by a child directory that `M` opens, or brought in by an
//! import written in `M`. Imports are not inherited by child modules.

use std::collections::HashMap;

use crate::ast::ImportDecl;

use super::super::types::{ModuleError, ModuleExports, ScopeViolation};
use super::super::unit::{ModuleSource, ModuleUnit};
use super::references::{Reference, program_references};
use super::tree::{ModuleTree, child_toward, is_ancestor};

/// Exports of the other units of the plan, by module path.
type UnitExports<'a> = HashMap<&'a str, &'a ModuleExports>;

/// Hierarchical module scope (ADR-0110) of every unit, checked once the
/// plan holds the exports of every unit an import may name.
pub(crate) fn validate_plan_scopes(units: &[ModuleUnit]) -> Result<(), ModuleError> {
    let exports = units
        .iter()
        .map(|unit| (unit.identity().module_path(), unit.exports()))
        .collect::<UnitExports<'_>>();
    units.iter().try_for_each(|unit| validate_unit_scope(unit, &exports))
}

fn validate_unit_scope(
    unit: &ModuleUnit,
    unit_exports: &UnitExports<'_>,
) -> Result<(), ModuleError> {
    let tree = ModuleTree::of_unit(unit);
    for source in unit.sources() {
        let Some(module_path) = source_module(&tree, source) else { continue };
        let checker = Checker { tree: &tree, module_path, unit_exports };
        for reference in program_references(source.program()) {
            checker.check(&reference).map_err(|reason| ModuleError::OutOfScopeReference {
                module: module_path.to_owned(),
                symbol: reference.path.clone(),
                reason,
                path: source.path().to_owned(),
                span: reference.span,
            })?;
        }
    }
    Ok(())
}

fn source_module<'a>(tree: &'a ModuleTree, source: &'a ModuleSource) -> Option<&'a str> {
    let module_path = match source.kind() {
        super::super::unit::ModuleSourceKind::Facade
        | super::super::unit::ModuleSourceKind::Sibling { .. } => tree.unit_path(),
        super::super::unit::ModuleSourceKind::ChildFacade { module_path }
        | super::super::unit::ModuleSourceKind::ChildSibling { module_path, .. } => module_path,
    };
    tree.contains(module_path).then_some(module_path)
}

struct Checker<'a> {
    tree: &'a ModuleTree,
    module_path: &'a str,
    unit_exports: &'a UnitExports<'a>,
}

impl Checker<'_> {
    fn check(&self, reference: &Reference) -> Result<(), ScopeViolation> {
        match reference.path.split_once("::") {
            None => self.check_name(&reference.path),
            Some((head, rest)) => self.check_qualified(head, rest),
        }
    }

    fn check_name(&self, name: &str) -> Result<(), ScopeViolation> {
        if let Some(owner) = self.tree.owner(name) {
            if self.sees_unqualified(owner, name) || self.opens_import_of(name) {
                return Ok(());
            }
            return Err(self.unqualified_violation(owner));
        }
        let suppliers =
            self.unit_imports().filter(|(_, import)| self.import_supplies(import, name));
        let mut imported_by = None;
        for (module_path, _) in suppliers {
            if module_path == self.module_path {
                return Ok(());
            }
            imported_by.get_or_insert(module_path);
        }
        imported_by.map_or(Ok(()), |imported_by| {
            Err(ScopeViolation::ImportNotInherited { imported_by: imported_by.to_owned() })
        })
    }

    fn check_qualified(&self, head: &str, rest: &str) -> Result<(), ScopeViolation> {
        let leaf = rest.rsplit("::").next().unwrap_or(rest);
        if let Some(import) = self.own_imports().find(|import| binding(import) == head) {
            let path = import.path.as_str();
            if self.tree.contains(path) && self.tree.owner(leaf).is_none() {
                return Err(ScopeViolation::NotDeclared { module: path.to_owned() });
            }
            return if !self.tree.contains(path)
                || self.visible_through(path, leaf)
                || self.qualifies_own_scope(path, leaf)
            {
                Ok(())
            } else {
                Err(self.unqualified_violation(self.tree.owner(leaf).unwrap_or(path)))
            };
        }
        if self.tree.owner(head).is_some() {
            return self.check_name(head);
        }
        match self.unit_imports().find(|(_, import)| binding(import) == head) {
            Some((imported_by, _)) => {
                Err(ScopeViolation::ImportNotInherited { imported_by: imported_by.to_owned() })
            }
            None => Ok(()),
        }
    }

    /// Declared here, declared by an ancestor, or exported by an opened child.
    fn sees_unqualified(&self, owner: &str, name: &str) -> bool {
        owner == self.module_path
            || is_ancestor(owner, self.module_path)
            || (is_ancestor(self.module_path, owner)
                && self.tree.exports(&child_toward(self.module_path, owner), name))
    }

    /// `name` reached through an import of the unit's own module `path`.
    fn visible_through(&self, path: &str, name: &str) -> bool {
        self.tree.exports(path, name)
            || (self.tree.declares(path, name)
                && (path == self.module_path || is_ancestor(path, self.module_path)))
    }

    /// `ancestor::name` where `name` is already in this module's scope: the
    /// qualification through an ancestor binding discloses nothing new.
    fn qualifies_own_scope(&self, path: &str, name: &str) -> bool {
        (path == self.module_path || is_ancestor(path, self.module_path))
            && self.tree.owner(name).is_some_and(|owner| self.sees_unqualified(owner, name))
    }

    fn opens_import_of(&self, name: &str) -> bool {
        self.own_imports().any(|import| {
            self.tree.contains(&import.path)
                && (import.is_open || import.selected.iter().any(|item| item == name))
                && self.visible_through(&import.path, name)
        })
    }

    fn unqualified_violation(&self, owner: &str) -> ScopeViolation {
        if is_ancestor(self.module_path, owner) {
            ScopeViolation::PrivateToChild { declared_in: owner.to_owned() }
        } else {
            ScopeViolation::NotInScope { declared_in: owner.to_owned() }
        }
    }

    fn import_supplies(&self, import: &ImportDecl, name: &str) -> bool {
        if self.tree.contains(&import.path) {
            return false;
        }
        if import.selected.iter().any(|item| item == name) {
            return true;
        }
        import.is_open
            && self
                .unit_exports
                .get(import.path.as_str())
                .is_some_and(|exports| exports.symbols.iter().any(|symbol| symbol.name == name))
    }

    fn own_imports(&self) -> impl Iterator<Item = &ImportDecl> {
        self.tree.module(self.module_path).into_iter().flat_map(|module| module.imports.iter())
    }

    fn unit_imports(&self) -> impl Iterator<Item = (&str, &ImportDecl)> {
        self.tree.modules().flat_map(|(path, module)| {
            module.imports.iter().map(move |import| (path.as_str(), import))
        })
    }
}

pub(super) fn binding(import: &ImportDecl) -> &str {
    import
        .alias
        .as_deref()
        .or_else(|| import.path.rsplit("::").next())
        .unwrap_or(import.path.as_str())
}
