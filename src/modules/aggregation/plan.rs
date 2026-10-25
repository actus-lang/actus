use std::collections::{HashMap, HashSet};

use crate::ast::{Program, TopLevelDecl};

use super::super::resolver::ModuleResolver;
use super::collision::{ExternalContract, external_contract};
use super::declarations::{append_internal_declarations, append_public_declarations};
use super::object_plan::{ModuleObjectPlan, build_object_plan};
use super::types::ModuleError;
use super::unit::ModuleUnit;
use super::validation::{declaration_import_binding, export_identity, validate_import_bindings};
use super::visibility::validate_import_visibility;

pub(super) use super::declarations::qualify_import_path;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleCompilationPlan {
    local: Program,
    caller: Program,
    units: Vec<ModuleUnit>,
    friend_modules: HashSet<String>,
}

impl ModuleCompilationPlan {
    pub fn local(&self) -> &Program {
        &self.local
    }

    pub fn caller(&self) -> &Program {
        &self.caller
    }

    pub fn with_caller_program(&self, caller: Program) -> Self {
        let mut plan = self.clone();
        plan.caller = caller;
        plan
    }

    pub fn units(&self) -> &[ModuleUnit] {
        &self.units
    }

    pub(crate) fn friend_modules(&self) -> &HashSet<String> {
        &self.friend_modules
    }

    pub(crate) fn has_friend_scope(&self) -> bool {
        !self.friend_modules.is_empty()
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
    build_compilation_plan_with_overlays_and_scope_check(
        None,
        program,
        resolver,
        overlays,
        true,
        HashSet::new(),
    )
}

pub fn build_test_compilation_plan(
    program: &Program,
    resolver: &ModuleResolver,
) -> Result<ModuleCompilationPlan, ModuleError> {
    let friend_modules = program
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            TopLevelDecl::Import(import) => Some(import.path.clone()),
            _ => None,
        })
        .collect();
    build_compilation_plan_with_overlays_and_scope_check(
        None,
        program,
        resolver,
        &HashMap::new(),
        true,
        friend_modules,
    )
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
        HashSet::new(),
    )
}

fn build_compilation_plan_with_overlays_and_scope_check(
    importer_module_path: Option<&str>,
    program: &Program,
    resolver: &ModuleResolver,
    overlays: &HashMap<std::path::PathBuf, String>,
    validate_root_import_bindings: bool,
    friend_modules: HashSet<String>,
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
        &friend_modules,
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
        friend_modules,
    })
}

#[allow(clippy::too_many_arguments)]
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
    friend_modules: &HashSet<String>,
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
            import.is_open,
            &import.selected,
            friend_modules,
        )?;
    }
    Ok(())
}

fn import_namespace(import: &crate::ast::ImportDecl) -> Option<String> {
    Some(
        import
            .alias
            .clone()
            .or_else(|| import.path.rsplit("::").next().map(str::to_owned))
            .unwrap_or_else(|| import.path.clone()),
    )
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

#[allow(clippy::too_many_arguments)]
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
    open_import: bool,
    selected: &[String],
    friend_modules: &HashSet<String>,
) -> Result<(), ModuleError> {
    let unit = super::unit::load_module_unit_with_overlays(resolver, module_path, overlays)?;
    let friend = friend_modules
        .iter()
        .any(|friend| module_path == friend || module_path.starts_with(&format!("{friend}::")));
    if !friend {
        validate_import_visibility(importer, module_path, &unit)?;
    }
    let first_load = imported_paths.insert(module_path.to_owned());
    if first_load {
        collect_imported_units(
            module_path,
            &unit,
            resolver,
            overlays,
            imported_paths,
            units,
            caller_declarations,
            appended_symbols,
            friend_modules,
        )?;
    }
    append_unit_declarations(
        &unit,
        module_path,
        caller_declarations,
        appended_symbols,
        namespace,
        friend,
        open_import,
        selected,
    );
    if first_load {
        units.push(unit);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn append_unit_declarations(
    unit: &ModuleUnit,
    module_path: &str,
    caller_declarations: &mut Vec<TopLevelDecl>,
    appended_symbols: &mut HashSet<(String, String, String)>,
    namespace: Option<&str>,
    friend: bool,
    open_import: bool,
    selected: &[String],
) {
    if friend {
        if selected.is_empty() {
            append_internal_declarations(
                caller_declarations,
                unit,
                module_path,
                namespace,
                appended_symbols,
            );
        }
        if open_import || !selected.is_empty() {
            if selected.is_empty() {
                append_internal_declarations(
                    caller_declarations,
                    unit,
                    module_path,
                    None,
                    appended_symbols,
                );
            } else {
                append_public_declarations(
                    caller_declarations,
                    unit,
                    module_path,
                    namespace,
                    selected,
                    appended_symbols,
                );
                append_public_declarations(
                    caller_declarations,
                    unit,
                    module_path,
                    None,
                    selected,
                    appended_symbols,
                );
            }
        }
        return;
    }

    append_public_declarations(
        caller_declarations,
        unit,
        module_path,
        namespace,
        selected,
        appended_symbols,
    );
    if open_import || !selected.is_empty() {
        append_public_declarations(
            caller_declarations,
            unit,
            module_path,
            None,
            selected,
            appended_symbols,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_imported_units(
    module_path: &str,
    unit: &ModuleUnit,
    resolver: &ModuleResolver,
    overlays: &HashMap<std::path::PathBuf, String>,
    imported_paths: &mut HashSet<String>,
    units: &mut Vec<ModuleUnit>,
    caller_declarations: &mut Vec<TopLevelDecl>,
    appended_symbols: &mut HashSet<(String, String, String)>,
    friend_modules: &HashSet<String>,
) -> Result<(), ModuleError> {
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
            import.is_open,
            &import.selected,
            friend_modules,
        )?;
    }
    Ok(())
}
