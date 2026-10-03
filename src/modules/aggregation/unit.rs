use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::Program;
use crate::lexer::scan;
use crate::parser::parse;

use super::super::resolver::{ModuleResolver, ResolvedModule};
use super::identity::ModuleNamespace;
use super::signature_visibility::validate_exported_signatures;
use super::types::{ModuleError, ModuleExports};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleIdentity {
    namespace: ModuleNamespace,
    module_path: String,
    source_key: String,
}

impl ModuleIdentity {
    pub fn namespace(&self) -> &ModuleNamespace {
        &self.namespace
    }

    pub fn module_path(&self) -> &str {
        &self.module_path
    }

    pub fn source_key(&self) -> &str {
        &self.source_key
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModuleSourceKind {
    Facade,
    Sibling { name: String },
    ChildFacade { module_path: String },
    ChildSibling { module_path: String, name: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleSource {
    path: PathBuf,
    kind: ModuleSourceKind,
    program: Program,
    fingerprint: String,
}

impl ModuleSource {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn kind(&self) -> &ModuleSourceKind {
        &self.kind
    }

    pub fn program(&self) -> &Program {
        &self.program
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleUnit {
    identity: ModuleIdentity,
    facade: PathBuf,
    sources: Vec<ModuleSource>,
    implementation: Program,
    exports: ModuleExports,
}

impl ModuleUnit {
    pub fn identity(&self) -> &ModuleIdentity {
        &self.identity
    }

    pub fn facade(&self) -> &Path {
        &self.facade
    }

    pub fn sources(&self) -> &[ModuleSource] {
        &self.sources
    }

    pub fn implementation(&self) -> &Program {
        &self.implementation
    }

    pub fn exports(&self) -> &ModuleExports {
        &self.exports
    }
}

pub fn load_module_unit(
    resolver: &ModuleResolver,
    module_path: &str,
) -> Result<ModuleUnit, ModuleError> {
    load_module_unit_with_overlays(resolver, module_path, &HashMap::new())
}

pub fn load_module_unit_with_overlays(
    resolver: &ModuleResolver,
    module_path: &str,
    overlays: &HashMap<PathBuf, String>,
) -> Result<ModuleUnit, ModuleError> {
    let resolved = resolver.resolve(module_path).map_err(ModuleError::Resolution)?;
    let implementation =
        super::parsing::parse_module_with_overlays(resolver, module_path, overlays)?;
    let exports = super::exports::exports_module_with_overlays(resolver, module_path, overlays)?;
    let sources = load_sources(resolver, &resolved, overlays)?;
    let source_key = sources
        .iter()
        .map(|source| format!("{}@{}", normalize(source.path()), source.fingerprint))
        .collect::<Vec<_>>()
        .join("|");
    let namespace =
        ModuleNamespace::from_module_path(module_path).map_err(ModuleError::Resolution)?;
    let unit = ModuleUnit {
        identity: ModuleIdentity { namespace, module_path: module_path.to_owned(), source_key },
        facade: resolved.facade().to_owned(),
        sources,
        implementation,
        exports,
    };
    validate_exported_signatures(&unit)?;
    Ok(unit)
}

fn load_sources(
    resolver: &ModuleResolver,
    resolved: &ResolvedModule,
    overlays: &HashMap<PathBuf, String>,
) -> Result<Vec<ModuleSource>, ModuleError> {
    let mut sources = Vec::new();
    load_module_sources(resolved, overlays, &mut sources, false)?;
    for child in resolved.children() {
        let child = resolver
            .resolve_child(
                resolved.module_path(),
                child.module_path().rsplit("::").next().unwrap_or_default(),
            )
            .map_err(ModuleError::Resolution)?;
        load_child_sources(resolver, &child, overlays, &mut sources)?;
    }
    Ok(sources)
}

fn load_module_sources(
    resolved: &ResolvedModule,
    overlays: &HashMap<PathBuf, String>,
    sources: &mut Vec<ModuleSource>,
    child: bool,
) -> Result<(), ModuleError> {
    let facade_kind = if child {
        ModuleSourceKind::ChildFacade { module_path: resolved.module_path().to_owned() }
    } else {
        ModuleSourceKind::Facade
    };
    sources.push(load_source(resolved.facade(), facade_kind, overlays)?);
    for sibling in resolved.siblings() {
        let name = sibling.file_stem().and_then(|stem| stem.to_str()).unwrap_or_default();
        let kind = if child {
            ModuleSourceKind::ChildSibling {
                module_path: resolved.module_path().to_owned(),
                name: name.to_owned(),
            }
        } else {
            ModuleSourceKind::Sibling { name: name.to_owned() }
        };
        sources.push(load_source(sibling, kind, overlays)?);
    }
    Ok(())
}

fn load_child_sources(
    resolver: &ModuleResolver,
    resolved: &ResolvedModule,
    overlays: &HashMap<PathBuf, String>,
    sources: &mut Vec<ModuleSource>,
) -> Result<(), ModuleError> {
    load_module_sources(resolved, overlays, sources, true)?;
    for child in resolved.children() {
        let child_path = child.module_path().rsplit("::").next().unwrap_or_default();
        let nested = resolver
            .resolve_child(resolved.module_path(), child_path)
            .map_err(ModuleError::Resolution)?;
        load_child_sources(resolver, &nested, overlays, sources)?;
    }
    Ok(())
}

fn load_source(
    path: &Path,
    kind: ModuleSourceKind,
    overlays: &HashMap<PathBuf, String>,
) -> Result<ModuleSource, ModuleError> {
    let source = overlays.get(path).cloned().map(Ok).unwrap_or_else(|| {
        fs::read_to_string(path).map_err(|error| ModuleError::Read {
            path: path.to_owned(),
            message: error.to_string(),
        })
    })?;
    let (tokens, errors) = scan(&source);
    if !errors.is_empty() {
        return Err(ModuleError::Lex { path: path.to_owned(), errors });
    }
    let program = parse(tokens)
        .map_err(|error| ModuleError::Parse { path: path.to_owned(), error: Box::new(error) })?;
    Ok(ModuleSource {
        path: path.to_owned(),
        kind,
        program,
        fingerprint: source_fingerprint(&source),
    })
}

fn source_fingerprint(source: &str) -> String {
    let mut first = 0xcbf29ce484222325u64;
    let mut second = 0x9e3779b185ebca87u64;
    for byte in source.as_bytes() {
        first = (first ^ u64::from(*byte)).wrapping_mul(0x100000001b3);
        second = (second ^ u64::from(*byte).rotate_left(13)).wrapping_mul(0x517cc1b727220a95);
    }
    format!("{first:016x}{second:016x}")
}

fn normalize(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
