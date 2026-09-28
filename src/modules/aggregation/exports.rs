use std::fs;
use std::path::PathBuf;

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
    let resolved = resolver.resolve(module_path).map_err(ModuleError::Resolution)?;
    let mut parsed = Vec::new();
    for source_path in resolved.source_files() {
        let source = fs::read_to_string(source_path).map_err(|error| ModuleError::Read {
            path: source_path.to_owned(),
            message: error.to_string(),
        })?;
        let (tokens, errors) = scan(&source);
        if !errors.is_empty() {
            return Err(ModuleError::Lex { path: source_path.to_owned(), errors });
        }
        let program = parse(tokens)
            .map_err(|error| ModuleError::Parse { path: source_path.to_owned(), error })?;
        parsed.push((source_path.to_owned(), program));
    }
    validate_open_siblings(module_path, resolved.facade(), resolved.siblings(), &parsed[0].1)?;
    Ok(collect_exports(&parsed))
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
