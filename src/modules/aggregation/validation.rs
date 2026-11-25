use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::ast::{ImportDecl, MetaAttribute, Program, TopLevelDecl};
use crate::lexer::SourceSpan;

use super::super::resolver::ResolvedChildModule;
use super::types::{DuplicateDeclaration, ModuleError, ModuleLocation};

type DeclarationLocation = (ModuleLocation, Option<String>);

pub(super) fn declarations_are_identical_without_spans(
    first: &TopLevelDecl,
    second: &TopLevelDecl,
) -> bool {
    let first = crate::formatter::format_program(&Program {
        file_metadata: Vec::new(),
        declarations: vec![first.clone()],
    });
    let second = crate::formatter::format_program(&Program {
        file_metadata: Vec::new(),
        declarations: vec![second.clone()],
    });
    first == second
}

pub(super) fn validate_import_bindings(program: &Program) -> Result<(), ModuleError> {
    let mut bindings = HashMap::new();
    for declaration in &program.declarations {
        let TopLevelDecl::Import(import) = declaration else { continue };
        let binding = import_binding(import);
        if let Some(first) = bindings.insert(binding.clone(), import.span) {
            return Err(ModuleError::ImportBindingCollision {
                binding,
                first,
                second: import.span,
            });
        }
    }
    Ok(())
}

pub(super) fn declaration_import_binding(declaration: &TopLevelDecl) -> Option<String> {
    let TopLevelDecl::Import(import) = declaration else { return None };
    Some(import_binding(import))
}

fn import_binding(import: &ImportDecl) -> String {
    import
        .alias
        .clone()
        .or_else(|| import.path.rsplit("::").next().map(str::to_owned))
        .unwrap_or_else(|| import.path.clone())
}

/// Rejects declarations that share a name across the given files, naming both
/// files (E1106). Production sources come first, so a test file that redeclares
/// a module name is the second location.
pub fn check_unique_declarations(files: &[(&Path, &Program)]) -> Result<(), ModuleError> {
    let mut locations = HashMap::new();
    for (path, program) in files {
        for declaration in &program.declarations {
            check_declaration_name(declaration, path, &mut locations)?;
        }
    }
    Ok(())
}

/// Rejects a `meta test` verb in a production source file (ADR-0121).
/// `tests_directory` is the `tests` directory of the module that owns the
/// file, or `None` for the root unit.
pub fn reject_test_verbs(
    path: &Path,
    program: &Program,
    tests_directory: Option<&Path>,
) -> Result<(), ModuleError> {
    let test = program.declarations.iter().find_map(|declaration| match declaration {
        TopLevelDecl::Verb(verb) if verb.metadata.contains(&MetaAttribute::Test) => Some(verb),
        _ => None,
    });
    match test {
        Some(verb) => Err(ModuleError::TestVerbInProduction {
            path: path.to_owned(),
            span: verb.span,
            verb: verb.name.clone(),
            tests_directory: tests_directory.map(Path::to_owned),
        }),
        None => Ok(()),
    }
}

pub(super) fn check_declaration_name(
    declaration: &TopLevelDecl,
    path: &Path,
    locations: &mut HashMap<(String, String), Vec<DeclarationLocation>>,
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
    let target = target_selector(declaration).map(str::to_owned);
    let entries = locations.entry(key).or_default();
    if let Some((first, _)) = entries.iter().find(|(_, first_target)| {
        !target_scopes_are_disjoint(first_target.as_deref(), target.as_deref())
    }) {
        return Err(ModuleError::DuplicateDeclaration(Box::new(DuplicateDeclaration {
            kind,
            name,
            first: first.clone(),
            second: location,
        })));
    }
    entries.push((location, target));
    Ok(())
}

pub(super) fn target_selector(declaration: &TopLevelDecl) -> Option<&str> {
    let metadata = match declaration {
        TopLevelDecl::Verb(value) => &value.metadata,
        TopLevelDecl::ExternalVerb(value) => &value.metadata,
        _ => return None,
    };
    metadata.iter().find_map(|attribute| match attribute {
        MetaAttribute::Target(target) => Some(target.as_str()),
        _ => None,
    })
}

pub(super) fn target_scopes_are_disjoint(first: Option<&str>, second: Option<&str>) -> bool {
    let (Some(first), Some(second)) = (first, second) else { return false };
    if first == second {
        return false;
    }
    matches!((first, second), ("windows", "unix" | "posix") | ("unix" | "posix", "windows"))
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
        TopLevelDecl::Serialize(value) => Some(("serialize", &value.name, value.span)),
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
    let mut reach = std::collections::HashMap::new();
    for declaration in &program.declarations {
        let TopLevelDecl::OpenSibling(sibling) = declaration else { continue };
        if !known.contains(sibling.name.as_str()) {
            return Err(ModuleError::UnknownSiblingModule {
                module: module_path.to_owned(),
                sibling: sibling.name.clone(),
                facade: facade.to_owned(),
            });
        }
        if reach
            .insert(sibling.name.as_str(), sibling.unit)
            .is_some_and(|unit| unit != sibling.unit)
        {
            return Err(ModuleError::ConflictingOpening {
                module: module_path.to_owned(),
                name: sibling.name.clone(),
                facade: facade.to_owned(),
                span: sibling.span,
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
    if let Some((kind, span)) = program.declarations.iter().find_map(runtime_declaration) {
        return Err(ModuleError::ConfigurationRuntimeDeclaration {
            module: module_path.to_owned(),
            path: source_path.to_owned(),
            kind: kind.to_owned(),
            span,
        });
    }
    Ok(())
}

fn runtime_declaration(declaration: &TopLevelDecl) -> Option<(&'static str, SourceSpan)> {
    match declaration {
        TopLevelDecl::Verb(value) => Some(("verb", value.span)),
        TopLevelDecl::ExternalVerb(value) => Some(("external verb", value.span)),
        TopLevelDecl::Role(value) => Some(("role", value.span)),
        TopLevelDecl::Perform(value) => Some(("perform", value.span)),
        TopLevelDecl::Constant(_)
        | TopLevelDecl::Struct(_)
        | TopLevelDecl::Pack(_)
        | TopLevelDecl::Serialize(_)
        | TopLevelDecl::Enum(_)
        | TopLevelDecl::OpenSibling(_)
        | TopLevelDecl::Import(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::declarations_are_identical_without_spans;
    use crate::ast::TopLevelDecl;

    fn declaration(source: &str, base_offset: usize) -> TopLevelDecl {
        let (tokens, errors) = crate::lexer::scan_at(source, base_offset);
        assert!(errors.is_empty(), "unexpected lexer errors: {errors:?}");
        crate::parser::parse(tokens)
            .unwrap_or_else(|error| panic!("{source}: declaration should parse: {error:?}"))
            .declarations
            .into_iter()
            .next()
            .expect("source should contain a declaration")
    }

    #[test]
    fn declaration_identity_ignores_source_spans_but_keeps_structure() {
        let first = declaration("enum Shared { Missing, }", 1);
        let second = declaration("enum Shared { Missing, }", 101);
        assert!(declarations_are_identical_without_spans(&first, &second));

        let verb = declaration("verb shared() -> Int { return 0; }", 1);
        let with_meta = declaration("meta deterministic verb shared() -> Int { return 0; }", 1);
        assert!(!declarations_are_identical_without_spans(&verb, &with_meta));

        let open = declaration("open verb shared() -> Int { return 0; }", 1);
        assert!(!declarations_are_identical_without_spans(&verb, &open));

        let targeted = declaration("meta target(\"unix\") verb shared() -> Int { return 0; }", 1);
        assert!(!declarations_are_identical_without_spans(&verb, &targeted));
    }
}
