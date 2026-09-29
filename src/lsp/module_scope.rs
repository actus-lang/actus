use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::ast::{Program, TopLevelDecl};
use crate::configuration::CompilerConfiguration;
use crate::modules::{ModuleResolver, exports_module_with_overlays};

pub(super) fn internal_symbols(program: &Program) -> Vec<String> {
    program.declarations.iter().filter_map(declaration_name).map(str::to_owned).collect()
}

pub(super) fn imported_public_symbols(
    uri: &str,
    program: &Program,
    overlays: &HashMap<PathBuf, String>,
) -> Vec<String> {
    let Some(path) = file_uri_to_path(uri) else { return Vec::new() };
    let Ok(configuration) = CompilerConfiguration::from_input_path_read_only(&path) else {
        return Vec::new();
    };
    let resolver = ModuleResolver::with_dependencies(
        configuration.source_root(),
        configuration.dependency_roots(),
    );
    program
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::Import(import) => Some(import.path.as_str()),
            _ => None,
        })
        .filter_map(|module| exports_module_with_overlays(&resolver, module, overlays).ok())
        .flat_map(|exports| exports.symbols.into_iter().map(|symbol| symbol.name))
        .collect()
}

pub(super) fn source_for_path(path: &Path, overlays: &HashMap<PathBuf, String>) -> Option<String> {
    overlays.get(path).cloned().or_else(|| std::fs::read_to_string(path).ok())
}

pub(super) fn file_uri_to_path(uri: &str) -> Option<PathBuf> {
    let path = uri.strip_prefix("file://")?;
    #[cfg(windows)]
    {
        Some(PathBuf::from(path.trim_start_matches('/').replace('/', "\\")))
    }
    #[cfg(not(windows))]
    Some(PathBuf::from(path))
}

fn declaration_name(declaration: &TopLevelDecl) -> Option<&str> {
    match declaration {
        TopLevelDecl::Verb(value) => Some(&value.name),
        TopLevelDecl::ExternalVerb(value) => Some(&value.name),
        TopLevelDecl::Struct(value) => Some(&value.name),
        TopLevelDecl::Enum(value) => Some(&value.name),
        TopLevelDecl::Role(value) => Some(&value.name),
        TopLevelDecl::Pack(value) => Some(&value.name),
        _ => None,
    }
}
