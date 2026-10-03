use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::ast::{Program, TopLevelDecl};
use crate::lexer::SourceSpan;

use super::super::resolver::ResolvedChildModule;
use super::types::{DuplicateDeclaration, ModuleError, ModuleLocation};

pub(super) fn check_declaration_name(
    declaration: &TopLevelDecl,
    path: &Path,
    locations: &mut HashMap<(String, String), ModuleLocation>,
) -> Result<(), ModuleError> {
    let (kind, name, span) = match declaration {
        TopLevelDecl::Perform(perform) => {
            ("perform".to_owned(), performance_identity(perform), perform.span)
        }
        _ => {
            let Some((kind, name, span)) = declaration_identity(declaration) else { return Ok(()) };
            (kind.to_owned(), name.to_owned(), span)
        }
    };
    let key = (kind.clone(), name.clone());
    let location = ModuleLocation { path: path.to_owned(), span };
    if let Some(first) = locations.insert(key, location.clone()) {
        return Err(ModuleError::DuplicateDeclaration(Box::new(DuplicateDeclaration {
            kind,
            name,
            first,
            second: location,
        })));
    }
    Ok(())
}

fn performance_identity(perform: &crate::ast::PerformDecl) -> String {
    format!("{} for {}", perform.role_name, canonical_type_name(&perform.target))
}

fn canonical_type_name(type_name: &crate::ast::TypeName) -> String {
    if type_name.arguments.is_empty() {
        return type_name.name.clone();
    }
    format!(
        "{}[{}]",
        type_name.name,
        type_name.arguments.iter().map(canonical_type_name).collect::<Vec<_>>().join(",")
    )
}

fn declaration_identity(declaration: &TopLevelDecl) -> Option<(&'static str, &String, SourceSpan)> {
    match declaration {
        TopLevelDecl::Constant(value) => Some(("const", &value.name, value.span)),
        TopLevelDecl::Struct(value) => Some(("struct", &value.name, value.span)),
        TopLevelDecl::Pack(value) => Some(("pack", &value.name, value.span)),
        TopLevelDecl::Enum(value) => Some(("enum", &value.name, value.span)),
        TopLevelDecl::Role(value) => Some(("role", &value.name, value.span)),
        TopLevelDecl::Verb(value) => Some(("verb", &value.name, value.span)),
        TopLevelDecl::ExternalVerb(value) => Some(("verb", &value.name, value.span)),
        TopLevelDecl::Perform(_) | TopLevelDecl::OpenSibling(_) | TopLevelDecl::Import(_) => None,
    }
}

pub(super) fn export_identity(declaration: &TopLevelDecl) -> Option<(&'static str, String)> {
    if let Some((kind, name, _)) = declaration_identity(declaration) {
        return Some((kind, name.clone()));
    }
    match declaration {
        TopLevelDecl::Perform(perform) => Some(("perform", performance_identity(perform))),
        _ => None,
    }
}

pub(super) fn validate_open_siblings(
    module_path: &str,
    facade: &Path,
    siblings: &[PathBuf],
    children: &[ResolvedChildModule],
    program: &Program,
) -> Result<(), ModuleError> {
    let mut known = siblings
        .iter()
        .filter_map(|path| path.file_stem().and_then(|stem| stem.to_str()))
        .collect::<std::collections::HashSet<_>>();
    known.extend(children.iter().filter_map(|child| child.module_path().rsplit("::").next()));
    for declaration in &program.declarations {
        if let TopLevelDecl::OpenSibling(sibling) = declaration
            && !known.contains(sibling.name.as_str())
        {
            return Err(ModuleError::UnknownSiblingModule {
                module: module_path.to_owned(),
                sibling: sibling.name.clone(),
                facade: facade.to_owned(),
            });
        }
    }
    Ok(())
}

pub(super) fn validate_configuration_source(
    module_path: &str,
    source_path: &Path,
    program: &crate::ast::Program,
) -> Result<(), ModuleError> {
    if module_path != "config" && !module_path.starts_with("config::") {
        return Ok(());
    }
    if let Some(import) = program.declarations.iter().find_map(|declaration| match declaration {
        TopLevelDecl::Import(import) => Some(import),
        _ => None,
    }) {
        return Err(ModuleError::ConfigurationImport {
            module: module_path.to_owned(),
            path: source_path.to_owned(),
            span: import.span,
        });
    }
    Ok(())
}
