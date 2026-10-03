use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::Program;
use crate::lexer::scan;
use crate::parser::parse;

use super::super::resolver::ModuleResolver;
use super::types::ModuleError;
use super::validation::{
    check_declaration_name, validate_configuration_source, validate_open_siblings,
};

pub fn parse_module(resolver: &ModuleResolver, module_path: &str) -> Result<Program, ModuleError> {
    let resolved = resolver.resolve(module_path).map_err(ModuleError::Resolution)?;
    let sources = resolved.source_files().map(Path::to_path_buf).collect::<Vec<_>>();
    let program = parse_sources(
        sources,
        |path| {
            fs::read_to_string(path).map_err(|error| ModuleError::Read {
                path: path.to_owned(),
                message: error.to_string(),
            })
        },
        module_path,
        resolved.facade(),
        resolved.siblings(),
        resolved.children(),
    )?;
    append_open_child_implementations(resolver, &HashMap::new(), &resolved, program)
}

pub fn parse_module_with_overlays(
    resolver: &ModuleResolver,
    module_path: &str,
    overlays: &HashMap<PathBuf, String>,
) -> Result<Program, ModuleError> {
    let resolved = resolver.resolve(module_path).map_err(ModuleError::Resolution)?;
    let sources = resolved.source_files().map(Path::to_path_buf).collect::<Vec<_>>();
    let program = parse_sources(
        sources,
        |path| source_for_path(path, overlays),
        module_path,
        resolved.facade(),
        resolved.siblings(),
        resolved.children(),
    )?;
    append_open_child_implementations(resolver, overlays, &resolved, program)
}

fn parse_sources<I, F>(
    source_paths: I,
    mut read_source: F,
    module_path: &str,
    facade: &Path,
    siblings: &[PathBuf],
    children: &[super::super::resolver::ResolvedChildModule],
) -> Result<Program, ModuleError>
where
    I: IntoIterator<Item = PathBuf>,
    F: FnMut(&Path) -> Result<String, ModuleError>,
{
    let mut declarations = Vec::new();
    let mut locations = HashMap::new();
    for (source_index, source_path) in source_paths.into_iter().enumerate() {
        let source = read_source(&source_path)?;
        let (tokens, errors) = scan(&source);
        if !errors.is_empty() {
            return Err(ModuleError::Lex { path: source_path, errors });
        }
        let program = parse(tokens).map_err(|error| ModuleError::Parse {
            path: source_path.clone(),
            error: Box::new(error),
        })?;
        validate_configuration_source(module_path, &source_path, &program)?;
        if source_index == 0 {
            validate_open_siblings(module_path, facade, siblings, children, &program)?;
        }
        for declaration in program.declarations {
            check_declaration_name(&declaration, &source_path, &mut locations)?;
            declarations.push(declaration);
        }
    }
    Ok(Program { file_metadata: Vec::new(), declarations })
}

fn append_open_child_implementations(
    resolver: &ModuleResolver,
    overlays: &HashMap<PathBuf, String>,
    resolved: &super::super::resolver::ResolvedModule,
    mut program: Program,
) -> Result<Program, ModuleError> {
    let opened_children = program
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            crate::ast::TopLevelDecl::OpenSibling(open) => Some(open.name.clone()),
            _ => None,
        })
        .collect::<std::collections::HashSet<_>>();
    for child in resolved.children() {
        let child_name = child.module_path().rsplit("::").next().unwrap_or_default();
        if opened_children.contains(child_name) {
            let child_program = parse_child_module_with_overlays(
                resolver,
                resolved.module_path(),
                child_name,
                overlays,
            )?;
            program.declarations.extend(child_program.declarations);
        }
    }
    Ok(program)
}

fn parse_child_module_with_overlays(
    resolver: &ModuleResolver,
    parent_module_path: &str,
    child_name: &str,
    overlays: &HashMap<PathBuf, String>,
) -> Result<Program, ModuleError> {
    let child =
        resolver.resolve_child(parent_module_path, child_name).map_err(ModuleError::Resolution)?;
    let sources = child.source_files().map(Path::to_path_buf).collect::<Vec<_>>();
    let program = parse_sources(
        sources,
        |path| source_for_path(path, overlays),
        child.module_path(),
        child.facade(),
        child.siblings(),
        child.children(),
    )?;
    append_open_child_implementations(resolver, overlays, &child, program)
}

fn source_for_path(
    path: &Path,
    overlays: &HashMap<PathBuf, String>,
) -> Result<String, ModuleError> {
    overlays.get(path).cloned().map(Ok).unwrap_or_else(|| {
        fs::read_to_string(path).map_err(|error| ModuleError::Read {
            path: path.to_owned(),
            message: error.to_string(),
        })
    })
}
