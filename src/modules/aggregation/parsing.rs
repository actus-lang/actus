use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::Program;
use crate::lexer::scan;
use crate::parser::parse;

use super::super::resolver::ModuleResolver;
use super::types::ModuleError;
use super::validation::{check_declaration_name, validate_open_siblings};

pub fn parse_module(resolver: &ModuleResolver, module_path: &str) -> Result<Program, ModuleError> {
    let resolved = resolver.resolve(module_path).map_err(ModuleError::Resolution)?;
    let sources = resolved.source_files().map(Path::to_path_buf).collect::<Vec<_>>();
    parse_sources(
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
    )
}

pub fn parse_module_with_overlays(
    resolver: &ModuleResolver,
    module_path: &str,
    overlays: &HashMap<PathBuf, String>,
) -> Result<Program, ModuleError> {
    let resolved = resolver.resolve(module_path).map_err(ModuleError::Resolution)?;
    let sources = resolved.source_files().map(Path::to_path_buf).collect::<Vec<_>>();
    parse_sources(
        sources,
        |path| source_for_path(path, overlays),
        module_path,
        resolved.facade(),
        resolved.siblings(),
    )
}

fn parse_sources<I, F>(
    source_paths: I,
    mut read_source: F,
    module_path: &str,
    facade: &Path,
    siblings: &[PathBuf],
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
        if source_index == 0 {
            validate_open_siblings(module_path, facade, siblings, &program)?;
        }
        for declaration in program.declarations {
            check_declaration_name(&declaration, &source_path, &mut locations)?;
            declarations.push(declaration);
        }
    }
    Ok(Program { file_metadata: Vec::new(), declarations })
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
