//! Imports of a child directory of another unit (ADR-0110).
//!
//! `import pkg::store::cache` names a directory that unit `pkg` compiles
//! when the facades `pkg` and `pkg::store` open it. Loading the directory
//! as a unit of its own would give its declarations a second canonical
//! identity (ADR-0108) and a second copy of their code. The import therefore
//! binds to the owning unit: it becomes `import pkg as cache`, and only the
//! child's exports are reachable through the binding. An `open` import selects
//! exactly those exports.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::ast::{ImportDecl, Program, TopLevelDecl};

use super::super::types::{ModuleError, ScopeViolation};
use super::check::binding;
use super::references::program_references;
use crate::modules::ModuleResolver;

/// The unit that compiles `module_path`: its shortest ancestor-or-self path
/// that resolves as a module with a facade (a bare namespace directory such
/// as `driver` in `driver::gpio` is not a unit).
pub(crate) fn owning_unit_path<'a>(resolver: &ModuleResolver, module_path: &'a str) -> &'a str {
    let first = if module_path.starts_with("std::") { 1 } else { 0 };
    module_path
        .match_indices("::")
        .skip(first)
        .map(|(end, _)| &module_path[..end])
        .find(|prefix| resolver.resolve(prefix).is_ok())
        .unwrap_or(module_path)
}

/// Every facade from `owner` down to `module_path` opens the next directory.
fn reject_unopened_child(
    context: &ImportContext<'_>,
    owner: &str,
    module_path: &str,
) -> Result<(), ModuleError> {
    let mut parent = owner.to_owned();
    for segment in module_path[owner.len() + 2..].split("::") {
        let resolved =
            context.resolver.resolve_for_aggregation(&parent).map_err(ModuleError::Resolution)?;
        let facade = resolved.facade();
        let source = match context.overlays.get(facade) {
            Some(source) => source.clone(),
            None => std::fs::read_to_string(facade).map_err(|error| ModuleError::Read {
                path: facade.to_owned(),
                message: error.to_string(),
            })?,
        };
        let program = crate::session::parse_source(context.resolver.session(), facade, &source)
            .program()
            .map_err(|error| ModuleError::frontend(facade, error))?;
        let opened = program.declarations.iter().any(|declaration| {
            // A child opened with `open unit` stays inside its unit (ADR-0111).
            matches!(declaration, TopLevelDecl::OpenSibling(open) if open.name == segment && !open.unit)
        });
        if !opened {
            return Err(ModuleError::Resolution(
                crate::modules::ModuleResolutionError::BypassesFacade {
                    module: module_path.to_owned(),
                    parent,
                    facade: facade.to_owned(),
                },
            ));
        }
        parent = format!("{parent}::{segment}");
    }
    Ok(())
}

/// Binds the root program's imports of other units' child modules
/// (ADR-0110); a friend child makes its owning unit a friend.
pub(crate) fn bind_root_child_imports(
    program: &Program,
    importer_module_path: Option<&str>,
    resolver: &ModuleResolver,
    overlays: &HashMap<PathBuf, String>,
    friend_modules: HashSet<String>,
) -> Result<(Program, HashSet<String>), ModuleError> {
    let mut program = program.clone();
    let context = ImportContext {
        importer: importer_module_path.unwrap_or(super::super::registry::ROOT_MODULE_PATH),
        resolver,
        overlays,
        report_path: Path::new(""),
        friends: &friend_modules,
    };
    let qualify = |path: &str| {
        importer_module_path
            .map(|module_path| super::super::declarations::qualify_import_path(module_path, path))
            .unwrap_or_else(|| path.to_owned())
    };
    bind_child_imports(&mut program, qualify, &context)?;
    let owners = friend_modules
        .iter()
        .map(|friend| owning_unit_path(resolver, friend).to_owned())
        .collect::<Vec<_>>();
    let mut friends = friend_modules;
    friends.extend(owners);
    Ok((program, friends))
}

pub(crate) struct ImportContext<'a> {
    pub(crate) importer: &'a str,
    pub(crate) resolver: &'a ModuleResolver,
    pub(crate) overlays: &'a HashMap<PathBuf, String>,
    pub(crate) report_path: &'a Path,
    /// Friend modules of a test plan see their module's private declarations.
    pub(crate) friends: &'a HashSet<String>,
}

pub(crate) fn bind_child_imports(
    program: &mut Program,
    qualify: impl Fn(&str) -> String,
    context: &ImportContext<'_>,
) -> Result<(), ModuleError> {
    let mut restricted = HashMap::<String, (String, BTreeSet<String>)>::new();
    for declaration in &mut program.declarations {
        let TopLevelDecl::Import(import) = declaration else { continue };
        let path = qualify(&import.path);
        let owner = owning_unit_path(context.resolver, &path);
        if owner == path {
            continue;
        }
        reject_unopened_child(context, owner, &path)?;
        let exports = super::super::exports::exports_module_with_overlays(
            context.resolver,
            &path,
            context.overlays,
        )?
        .symbols
        .into_iter()
        .map(|symbol| symbol.name)
        .collect::<BTreeSet<_>>();
        let friend = context.friends.contains(&path);
        if !friend && let Some(name) = import.selected.iter().find(|name| !exports.contains(*name))
        {
            return Err(not_exported(context, name, &path, import.span));
        }
        let name = binding(import).to_owned();
        let selected = if import.is_open {
            exports.iter().cloned().collect()
        } else {
            import.selected.clone()
        };
        if !friend {
            restricted.insert(name.clone(), (path.clone(), exports));
        }
        *import = ImportDecl {
            path: owner.to_owned(),
            alias: Some(name),
            is_open: false,
            selected,
            grouped: import.grouped,
            span: import.span,
        };
    }
    if restricted.is_empty() {
        return Ok(());
    }
    for reference in program_references(program) {
        let Some((head, rest)) = reference.path.split_once("::") else { continue };
        let Some((path, exports)) = restricted.get(head) else { continue };
        let leaf = rest.rsplit("::").next().unwrap_or(rest);
        if !exports.contains(leaf) {
            return Err(not_exported(context, &reference.path, path, reference.span));
        }
    }
    Ok(())
}

fn not_exported(
    context: &ImportContext<'_>,
    symbol: &str,
    module: &str,
    span: crate::lexer::SourceSpan,
) -> ModuleError {
    ModuleError::OutOfScopeReference {
        module: context.importer.to_owned(),
        symbol: symbol.to_owned(),
        reason: ScopeViolation::NotExported { module: module.to_owned() },
        path: context.report_path.to_owned(),
        span,
    }
}
