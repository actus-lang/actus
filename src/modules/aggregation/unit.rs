use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::Program;
use crate::lexer::scan;
use crate::parser::parse;

use super::super::resolver::ModuleResolver;
use super::exports::exports_module;
use super::parsing::parse_module;
use super::types::{ModuleError, ModuleExports};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleIdentity {
    module_path: String,
    source_key: String,
}

impl ModuleIdentity {
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleSource {
    path: PathBuf,
    kind: ModuleSourceKind,
    program: Program,
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
    let resolved = resolver.resolve(module_path).map_err(ModuleError::Resolution)?;
    let implementation = parse_module(resolver, module_path)?;
    let exports = exports_module(resolver, module_path)?;
    let sources = load_sources(resolved.facade(), resolved.siblings())?;
    let source_key =
        sources.iter().map(|source| normalize(source.path())).collect::<Vec<_>>().join("|");
    Ok(ModuleUnit {
        identity: ModuleIdentity { module_path: module_path.to_owned(), source_key },
        facade: resolved.facade().to_owned(),
        sources,
        implementation,
        exports,
    })
}

fn load_sources(facade: &Path, siblings: &[PathBuf]) -> Result<Vec<ModuleSource>, ModuleError> {
    let mut sources = Vec::with_capacity(siblings.len() + 1);
    sources.push(load_source(facade, ModuleSourceKind::Facade)?);
    for sibling in siblings {
        let name = sibling.file_stem().and_then(|stem| stem.to_str()).unwrap_or_default();
        sources.push(load_source(sibling, ModuleSourceKind::Sibling { name: name.to_owned() })?);
    }
    Ok(sources)
}

fn load_source(path: &Path, kind: ModuleSourceKind) -> Result<ModuleSource, ModuleError> {
    let source = fs::read_to_string(path)
        .map_err(|error| ModuleError::Read { path: path.to_owned(), message: error.to_string() })?;
    let (tokens, errors) = scan(&source);
    if !errors.is_empty() {
        return Err(ModuleError::Lex { path: path.to_owned(), errors });
    }
    let program =
        parse(tokens).map_err(|error| ModuleError::Parse { path: path.to_owned(), error })?;
    Ok(ModuleSource { path: path.to_owned(), kind, program })
}

fn normalize(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
