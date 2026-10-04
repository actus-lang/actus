use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::{Program, TopLevelDecl};
use crate::lexer::scan;
use crate::parser::parse;

use super::super::resolver::ModuleResolver;
use super::types::{ExportedSymbol, ModuleError, ModuleExports};
use super::validation::{export_identity, validate_open_siblings};

pub fn exports_module(
    resolver: &ModuleResolver,
    module_path: &str,
) -> Result<ModuleExports, ModuleError> {
    exports_module_with_overlays(resolver, module_path, &HashMap::new())
}

pub fn exports_module_with_overlays(
    resolver: &ModuleResolver,
    module_path: &str,
    overlays: &HashMap<PathBuf, String>,
) -> Result<ModuleExports, ModuleError> {
    let resolved =
        resolver.resolve_for_aggregation(module_path).map_err(ModuleError::Resolution)?;
    let parsed = parse_sources(resolved.source_files(), overlays)?;
    validate_open_siblings(
        module_path,
        resolved.facade(),
        resolved.siblings(),
        resolved.children(),
        &parsed[0].1,
    )?;
    let mut exports = ModuleExports { symbols: Vec::new() };
    append_exports(&mut exports, collect_exports(&parsed), module_path, module_path)?;
    let opened_children = parsed[0]
        .1
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::OpenSibling(open) => Some(open.name.as_str()),
            _ => None,
        })
        .collect::<std::collections::HashSet<_>>();
    for child in resolved.children() {
        let child_name = child.module_path().rsplit("::").next().unwrap_or_default();
        if opened_children.contains(child_name) {
            let child_exports =
                exports_child_module_with_overlays(resolver, module_path, child_name, overlays)?;
            append_exports(&mut exports, child_exports, module_path, child.module_path())?;
        }
    }
    Ok(exports)
}

fn exports_child_module_with_overlays(
    resolver: &ModuleResolver,
    parent_module_path: &str,
    child_name: &str,
    overlays: &HashMap<PathBuf, String>,
) -> Result<ModuleExports, ModuleError> {
    let child =
        resolver.resolve_child(parent_module_path, child_name).map_err(ModuleError::Resolution)?;
    let parsed = parse_sources(child.source_files(), overlays)?;
    validate_open_siblings(
        child.module_path(),
        child.facade(),
        child.siblings(),
        child.children(),
        &parsed[0].1,
    )?;
    let mut exports = ModuleExports { symbols: Vec::new() };
    append_exports(
        &mut exports,
        collect_exports(&parsed),
        child.module_path(),
        child.module_path(),
    )?;
    let opened_children = parsed[0]
        .1
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::OpenSibling(open) => Some(open.name.as_str()),
            _ => None,
        })
        .collect::<std::collections::HashSet<_>>();
    for nested in child.children() {
        let nested_name = nested.module_path().rsplit("::").next().unwrap_or_default();
        if opened_children.contains(nested_name) {
            let nested_exports = exports_child_module_with_overlays(
                resolver,
                child.module_path(),
                nested_name,
                overlays,
            )?;
            append_exports(
                &mut exports,
                nested_exports,
                child.module_path(),
                nested.module_path(),
            )?;
        }
    }
    Ok(exports)
}

fn parse_sources<'a, I>(
    source_paths: I,
    overlays: &HashMap<PathBuf, String>,
) -> Result<Vec<(PathBuf, Program)>, ModuleError>
where
    I: IntoIterator<Item = &'a Path>,
{
    source_paths
        .into_iter()
        .map(|source_path| {
            let source = overlays.get(source_path).cloned().map(Ok).unwrap_or_else(|| {
                fs::read_to_string(source_path).map_err(|error| ModuleError::Read {
                    path: source_path.to_owned(),
                    message: error.to_string(),
                })
            })?;
            let (tokens, errors) = scan(&source);
            if !errors.is_empty() {
                return Err(ModuleError::Lex { path: source_path.to_owned(), errors });
            }
            let program = parse(tokens).map_err(|error| ModuleError::Parse {
                path: source_path.to_owned(),
                error: Box::new(error),
            })?;
            Ok((source_path.to_owned(), program))
        })
        .collect()
}

fn append_exports(
    target: &mut ModuleExports,
    incoming: ModuleExports,
    first_module: &str,
    second_module: &str,
) -> Result<(), ModuleError> {
    let mut incoming_names = HashSet::new();
    for symbol in incoming.symbols {
        let identity = (symbol.kind.clone(), symbol.name.clone());
        if target
            .symbols
            .iter()
            .any(|existing| (existing.kind.clone(), existing.name.clone()) == identity)
            || !incoming_names.insert(identity)
        {
            return Err(ModuleError::SymbolCollision {
                symbol: format!("{} `{}`", symbol.kind, symbol.name),
                first_module: first_module.to_owned(),
                second_module: second_module.to_owned(),
            });
        }
        target.symbols.push(symbol);
    }
    Ok(())
}

fn collect_exports(parsed: &[(PathBuf, Program)]) -> ModuleExports {
    let facade_open = parsed[0]
        .1
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::OpenSibling(sibling) => Some(sibling.name.as_str()),
            _ => None,
        })
        .collect::<std::collections::HashSet<_>>();
    let mut symbols = Vec::new();
    for (index, (path, program)) in parsed.iter().enumerate() {
        let sibling_name = path.file_stem().and_then(|stem| stem.to_str());
        if index > 0 && sibling_name.is_some_and(|name| facade_open.contains(name)) {
            symbols.extend(program.declarations.iter().filter_map(|declaration| {
                let (kind, name) = export_identity(declaration)?;
                declaration_is_open(declaration).then(|| ExportedSymbol {
                    kind: kind.to_owned(),
                    name,
                    source: path.clone(),
                })
            }));
        }
    }
    ModuleExports { symbols }
}

fn declaration_is_open(declaration: &TopLevelDecl) -> bool {
    match declaration {
        TopLevelDecl::Constant(value) => value.is_open,
        TopLevelDecl::Verb(value) => value.is_open,
        TopLevelDecl::ExternalVerb(value) => value.is_open,
        TopLevelDecl::Struct(value) => value.is_open,
        TopLevelDecl::Pack(value) => value.is_open,
        TopLevelDecl::Enum(value) => value.is_open,
        TopLevelDecl::Role(value) => value.is_open,
        TopLevelDecl::Perform(value) => value.is_open,
        TopLevelDecl::OpenSibling(_) | TopLevelDecl::Import(_) => false,
    }
}
