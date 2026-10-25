use std::collections::HashSet;

use crate::ast::{ExternalVerbDecl, ForeignAbi, TopLevelDecl};

use super::collision::external_contract;
use super::unit::ModuleUnit;
use super::validation::export_identity;

pub(super) fn append_internal_declarations(
    target: &mut Vec<TopLevelDecl>,
    unit: &ModuleUnit,
    module_path: &str,
    namespace: Option<&str>,
    appended_symbols: &mut HashSet<(String, String, String)>,
) {
    for declaration in &unit.implementation().declarations {
        if matches!(declaration, TopLevelDecl::OpenSibling(_) | TopLevelDecl::Import(_)) {
            continue;
        }
        let Some((kind, name)) = export_identity(declaration) else { continue };
        if kind != "verb"
            && target
                .iter()
                .any(|existing| export_identity(existing).as_ref() == Some(&(kind, name.clone())))
        {
            continue;
        }
        let key = (
            module_path.to_owned(),
            format!("internal:{kind}:{name}"),
            namespace.unwrap_or_default().to_owned(),
        );
        if appended_symbols.insert(key) {
            target.push(caller_declaration(
                declaration,
                namespace,
                Some(unit.identity().namespace().symbol_prefix()),
            ));
        }
    }
}

pub(super) fn qualify_import_path(importer: &str, imported: &str) -> String {
    if importer.starts_with("std::") && !imported.contains("::") {
        format!("std::{imported}")
    } else {
        imported.to_owned()
    }
}

pub(super) fn append_public_declarations(
    target: &mut Vec<TopLevelDecl>,
    unit: &ModuleUnit,
    module_path: &str,
    namespace: Option<&str>,
    appended_symbols: &mut HashSet<(String, String, String)>,
) {
    for declaration in &unit.implementation().declarations {
        let TopLevelDecl::Import(import) = declaration else { continue };
        if import.is_open {
            continue;
        }
        let binding = import
            .alias
            .clone()
            .or_else(|| import.path.rsplit("::").next().map(str::to_owned))
            .unwrap_or_else(|| import.path.clone());
        let key = (module_path.to_owned(), format!("import:{binding}"), String::new());
        if appended_symbols.insert(key) {
            let mut imported = import.clone();
            imported.path = qualify_import_path(module_path, &import.path);
            target.push(TopLevelDecl::Import(imported));
        }
    }
    for declaration in &unit.implementation().declarations {
        let Some((kind, name)) = export_identity(declaration) else { continue };
        if !unit.exports().contains(kind, &name) {
            continue;
        }
        let symbol_namespace = (kind == "verb").then(|| namespace.unwrap_or_default().to_owned());
        let key = (
            module_path.to_owned(),
            format!("{kind}:{name}"),
            symbol_namespace.unwrap_or_default(),
        );
        if !appended_symbols.insert(key) {
            continue;
        }
        let imported = caller_declaration(
            declaration,
            namespace,
            Some(unit.identity().namespace().symbol_prefix()),
        );
        if !has_compatible_external_declaration(target, &imported)
            && !has_identical_imported_declaration(target, &imported)
        {
            target.push(imported);
        }
    }
}

fn has_identical_imported_declaration(
    declarations: &[TopLevelDecl],
    candidate: &TopLevelDecl,
) -> bool {
    let Some(candidate_identity) = export_identity(candidate) else { return false };
    declarations.iter().any(|declaration| {
        export_identity(declaration).as_ref() == Some(&candidate_identity)
            && declaration == candidate
    })
}

fn has_compatible_external_declaration(
    declarations: &[TopLevelDecl],
    candidate: &TopLevelDecl,
) -> bool {
    let Some((candidate_kind, candidate_name)) = export_identity(candidate) else { return false };
    let Some(candidate_contract) = external_contract(candidate) else { return false };
    declarations.iter().any(|declaration| {
        let Some((kind, name)) = export_identity(declaration) else { return false };
        kind == candidate_kind
            && name == candidate_name
            && external_contract(declaration).as_ref() == Some(&candidate_contract)
    })
}

fn caller_declaration(
    declaration: &TopLevelDecl,
    namespace: Option<&str>,
    module_symbol_namespace: Option<&str>,
) -> TopLevelDecl {
    match declaration {
        TopLevelDecl::Perform(perform) => {
            let mut imported = perform.clone();
            imported.module_import = true;
            imported.module_import_namespace = module_symbol_namespace.map(str::to_owned);
            TopLevelDecl::Perform(imported)
        }
        TopLevelDecl::Verb(verb) => TopLevelDecl::ExternalVerb(ExternalVerbDecl {
            is_open: verb.is_open,
            doc: verb.doc.clone(),
            contract: verb.contract.clone(),
            unsafe_boundary: false,
            module_import: true,
            abi: ForeignAbi::C,
            metadata: verb.metadata.clone(),
            name: qualified_name(namespace, &verb.name),
            native_symbol: namespace.map(|_| verb.name.clone()),
            generic_parameters: verb.generic_parameters.clone(),
            params: verb.params.clone(),
            return_type: verb.return_type.clone(),
            span: verb.span,
        }),
        TopLevelDecl::ExternalVerb(verb) => {
            let mut imported = verb.clone();
            imported.unsafe_boundary = false;
            imported.module_import = true;
            if let Some(namespace) = namespace {
                imported.native_symbol =
                    Some(imported.native_symbol.clone().unwrap_or_else(|| imported.name.clone()));
                imported.name = qualified_name(Some(namespace), &imported.name);
            }
            TopLevelDecl::ExternalVerb(imported)
        }
        _ => declaration.clone(),
    }
}

fn qualified_name(namespace: Option<&str>, name: &str) -> String {
    namespace.map_or_else(|| name.to_owned(), |namespace| format!("{namespace}::{name}"))
}
