use std::collections::{HashMap, HashSet};

use crate::ast::{ExternalVerbDecl, ForeignAbi, Program, TopLevelDecl};

use super::super::resolver::ModuleResolver;
use super::collision::{ExternalContract, external_contract};
use super::object_plan::{ModuleObjectPlan, build_object_plan};
use super::types::ModuleError;
use super::unit::ModuleUnit;
use super::validation::{declaration_import_binding, export_identity, validate_import_bindings};
use super::visibility::validate_import_visibility;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleCompilationPlan {
    local: Program,
    caller: Program,
    units: Vec<ModuleUnit>,
}

impl ModuleCompilationPlan {
    pub fn local(&self) -> &Program {
        &self.local
    }

    pub fn caller(&self) -> &Program {
        &self.caller
    }

    pub fn units(&self) -> &[ModuleUnit] {
        &self.units
    }

    pub fn object_plan(&self) -> Result<ModuleObjectPlan, ModuleError> {
        build_object_plan(self, &[])
    }

    pub fn object_plan_with_generic_types(
        &self,
        generic_types: &[crate::ast::TypeName],
    ) -> Result<ModuleObjectPlan, ModuleError> {
        build_object_plan(self, generic_types)
    }

    pub fn implementation(&self) -> Program {
        let mut declarations = self.local.declarations.clone();
        let mut import_bindings =
            declarations.iter().filter_map(declaration_import_binding).collect::<HashSet<_>>();
        append_unique_imports(&mut declarations, &mut import_bindings, &self.caller.declarations);
        declarations.extend(
            self.caller
                .declarations
                .iter()
                .filter(|declaration| {
                    matches!(
                        declaration,
                        TopLevelDecl::ExternalVerb(verb)
                            if verb.module_import && verb.name.contains("::")
                    )
                })
                .cloned(),
        );
        for unit in &self.units {
            append_unique_imports(
                &mut declarations,
                &mut import_bindings,
                &unit.implementation().declarations,
            );
            declarations.extend(
                unit.implementation()
                    .declarations
                    .iter()
                    .filter(|declaration| {
                        !matches!(
                            declaration,
                            TopLevelDecl::OpenSibling(_) | TopLevelDecl::Import(_)
                        )
                    })
                    .cloned(),
            );
        }
        Program { file_metadata: Vec::new(), declarations }
    }
}

fn append_unique_imports(
    declarations: &mut Vec<TopLevelDecl>,
    bindings: &mut HashSet<String>,
    source: &[TopLevelDecl],
) {
    for declaration in source {
        let Some(binding) = declaration_import_binding(declaration) else { continue };
        if bindings.insert(binding) {
            declarations.push(declaration.clone());
        }
    }
}

pub fn build_compilation_plan(
    program: &Program,
    resolver: &ModuleResolver,
) -> Result<ModuleCompilationPlan, ModuleError> {
    build_compilation_plan_with_overlays(program, resolver, &HashMap::new())
}

pub fn build_compilation_plan_with_overlays(
    program: &Program,
    resolver: &ModuleResolver,
    overlays: &HashMap<std::path::PathBuf, String>,
) -> Result<ModuleCompilationPlan, ModuleError> {
    build_compilation_plan_with_overlays_and_scope_check(None, program, resolver, overlays, true)
}

pub(super) fn build_module_compilation_plan_with_overlays(
    module_path: &str,
    program: &Program,
    resolver: &ModuleResolver,
    overlays: &HashMap<std::path::PathBuf, String>,
) -> Result<ModuleCompilationPlan, ModuleError> {
    build_compilation_plan_with_overlays_and_scope_check(
        Some(module_path),
        program,
        resolver,
        overlays,
        false,
    )
}

fn build_compilation_plan_with_overlays_and_scope_check(
    importer_module_path: Option<&str>,
    program: &Program,
    resolver: &ModuleResolver,
    overlays: &HashMap<std::path::PathBuf, String>,
    validate_root_import_bindings: bool,
) -> Result<ModuleCompilationPlan, ModuleError> {
    let mut caller_declarations = Vec::new();
    let mut local_declarations = Vec::new();
    let mut units = Vec::new();
    let mut imported_paths = HashSet::new();
    let mut appended_symbols = HashSet::new();
    if validate_root_import_bindings
        && !program
            .declarations
            .iter()
            .any(|declaration| matches!(declaration, TopLevelDecl::OpenSibling(_)))
    {
        validate_import_bindings(program)?;
    }
    collect_root_imports(
        program,
        importer_module_path,
        resolver,
        overlays,
        &mut imported_paths,
        &mut units,
        &mut local_declarations,
        &mut caller_declarations,
        &mut appended_symbols,
    )?;
    validate_symbol_collisions(&local_declarations, &units)?;
    Ok(ModuleCompilationPlan {
        local: Program {
            file_metadata: program.file_metadata.clone(),
            declarations: local_declarations,
        },
        caller: Program {
            file_metadata: program.file_metadata.clone(),
            declarations: caller_declarations,
        },
        units,
    })
}

fn collect_root_imports(
    program: &Program,
    importer_module_path: Option<&str>,
    resolver: &ModuleResolver,
    overlays: &HashMap<std::path::PathBuf, String>,
    imported_paths: &mut HashSet<String>,
    units: &mut Vec<ModuleUnit>,
    local_declarations: &mut Vec<TopLevelDecl>,
    caller_declarations: &mut Vec<TopLevelDecl>,
    appended_symbols: &mut HashSet<(String, String, String)>,
) -> Result<(), ModuleError> {
    for declaration in &program.declarations {
        let TopLevelDecl::Import(import) = declaration else {
            local_declarations.push(declaration.clone());
            caller_declarations.push(declaration.clone());
            continue;
        };
        caller_declarations.push(declaration.clone());
        let namespace = import_namespace(import);
        let import_path = importer_module_path
            .map(|module_path| qualify_import_path(module_path, &import.path))
            .unwrap_or_else(|| import.path.clone());
        collect_unit(
            &import_path,
            program,
            resolver,
            overlays,
            imported_paths,
            units,
            caller_declarations,
            appended_symbols,
            namespace.as_deref(),
        )?;
    }
    Ok(())
}

fn import_namespace(import: &crate::ast::ImportDecl) -> Option<String> {
    (!import.is_open).then(|| {
        import
            .alias
            .clone()
            .or_else(|| import.path.rsplit("::").next().map(str::to_owned))
            .unwrap_or_else(|| import.path.clone())
    })
}

fn validate_symbol_collisions(
    local_declarations: &[TopLevelDecl],
    units: &[ModuleUnit],
) -> Result<(), ModuleError> {
    let mut symbols = HashMap::new();
    record_symbols("<root>", local_declarations, &mut symbols)?;
    for unit in units {
        record_symbols(
            unit.identity().module_path(),
            &unit.implementation().declarations,
            &mut symbols,
        )?;
    }
    Ok(())
}

fn record_symbols(
    module: &str,
    declarations: &[TopLevelDecl],
    symbols: &mut HashMap<(String, String), SymbolRecord>,
) -> Result<(), ModuleError> {
    for declaration in declarations {
        let Some((kind, name)) = export_identity(declaration) else { continue };
        let key = (kind.to_owned(), name.clone());
        let record = SymbolRecord {
            module: module.to_owned(),
            external_contract: external_contract(declaration),
            declaration: declaration.clone(),
        };
        if let Some(first) = symbols.get(&key)
            && first.module != module
        {
            if first.declaration == *declaration {
                continue;
            }
            if first.external_contract.is_some()
                && first.external_contract == record.external_contract
            {
                continue;
            }
            return Err(ModuleError::SymbolCollision {
                symbol: if record.external_contract.is_some() {
                    format!("native symbol `{name}`")
                } else {
                    format!("{kind} `{name}`")
                },
                first_module: first.module.clone(),
                second_module: module.to_owned(),
            });
        }
        symbols.insert(key, record);
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SymbolRecord {
    module: String,
    external_contract: Option<ExternalContract>,
    declaration: TopLevelDecl,
}

fn collect_unit(
    module_path: &str,
    importer: &Program,
    resolver: &ModuleResolver,
    overlays: &HashMap<std::path::PathBuf, String>,
    imported_paths: &mut HashSet<String>,
    units: &mut Vec<ModuleUnit>,
    caller_declarations: &mut Vec<TopLevelDecl>,
    appended_symbols: &mut HashSet<(String, String, String)>,
    namespace: Option<&str>,
) -> Result<(), ModuleError> {
    let unit = super::unit::load_module_unit_with_overlays(resolver, module_path, overlays)?;
    validate_import_visibility(importer, module_path, &unit)?;
    let first_load = imported_paths.insert(module_path.to_owned());
    if first_load {
        for declaration in &unit.implementation().declarations {
            let TopLevelDecl::Import(import) = declaration else { continue };
            let import_path = qualify_import_path(module_path, &import.path);
            let imported_namespace = import_namespace(import);
            collect_unit(
                &import_path,
                unit.implementation(),
                resolver,
                overlays,
                imported_paths,
                units,
                caller_declarations,
                appended_symbols,
                imported_namespace.as_deref(),
            )?;
        }
    }
    append_public_declarations(
        caller_declarations,
        &unit,
        module_path,
        namespace,
        appended_symbols,
    );
    if first_load {
        units.push(unit);
    }
    Ok(())
}

pub(super) fn qualify_import_path(importer: &str, imported: &str) -> String {
    if importer.starts_with("std::") && !imported.contains("::") {
        format!("std::{imported}")
    } else {
        imported.to_owned()
    }
}

fn append_public_declarations(
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
