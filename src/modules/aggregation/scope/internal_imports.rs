//! Imports that name a directory module of the importing unit (ADR-0110).
//!
//! Such an import binds to the unit that already compiles the module; it
//! never loads a second unit for part of the same source tree. Open and
//! selected imports add no declarations because the unit already holds every
//! declaration; the scope check decides whether the spelling is in scope. A
//! namespace import only qualifies spellings, which are rewritten to the
//! unit-local names (`qualified.rs`).

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{Program, TopLevelDecl};
use crate::lexer::SourceSpan;

use super::super::types::ModuleError;
use super::check::binding;
use super::tree::ModuleTree;

pub(crate) fn bind_internal_imports(
    tree: &ModuleTree,
    implementation: &mut Program,
) -> Result<(), ModuleError> {
    let mut bindings = BTreeMap::<String, (String, SourceSpan)>::new();
    let mut declarations = Vec::with_capacity(implementation.declarations.len());
    for declaration in std::mem::take(&mut implementation.declarations) {
        let TopLevelDecl::Import(import) = &declaration else {
            declarations.push(declaration);
            continue;
        };
        let path = super::super::declarations::qualify_import_path(tree.unit_path(), &import.path);
        let internal = tree.contains(&path);
        // Every import form binds its last path segment (or alias) as a
        // namespace, `open` and selecting imports included.
        let name = binding(import).to_owned();
        match bindings.get(&name) {
            Some((bound, first)) if *bound != path && (tree.contains(bound) || internal) => {
                return Err(ModuleError::ImportBindingCollision {
                    binding: name,
                    first: *first,
                    second: import.span,
                });
            }
            Some(_) => {}
            None => {
                bindings.insert(name, (path, import.span));
            }
        }
        if internal {
            continue;
        }
        declarations.push(declaration);
    }
    let internal = bindings
        .into_iter()
        .filter(|(_, (path, _))| tree.contains(path))
        .map(|(name, _)| name)
        .collect::<BTreeSet<_>>();
    if !internal.is_empty() {
        let qualifiers = super::qualified::Qualifiers { tree, bindings: &internal };
        declarations.iter_mut().for_each(|declaration| qualifiers.declaration(declaration));
    }
    implementation.declarations = declarations;
    Ok(())
}
