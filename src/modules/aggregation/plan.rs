use std::collections::{HashMap, HashSet};

use crate::ast::{
    DispatchMode, ExternalVerbDecl, ForeignAbi, GenericParamKind, Program, ReturnAccess, Role,
    TopLevelDecl,
};

use super::super::resolver::ModuleResolver;
use super::object_plan::{ModuleObjectPlan, build_object_plan};
use super::types::ModuleError;
use super::unit::ModuleUnit;
use super::validation::{export_identity, validate_import_bindings};
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
        declarations.extend(
            self.caller
                .declarations
                .iter()
                .filter(|declaration| matches!(declaration, TopLevelDecl::Import(_)))
                .cloned(),
        );
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
            declarations.extend(
                unit.implementation()
                    .declarations
                    .iter()
                    .filter(|declaration| {
                        !matches!(
                            declaration,
                            TopLevelDecl::Import(_) | TopLevelDecl::OpenSibling(_)
                        )
                    })
                    .cloned(),
            );
        }
        Program { file_metadata: Vec::new(), declarations }
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
    build_compilation_plan_with_overlays_and_scope_check(program, resolver, overlays, true)
}

pub(super) fn build_module_compilation_plan_with_overlays(
    program: &Program,
    resolver: &ModuleResolver,
    overlays: &HashMap<std::path::PathBuf, String>,
) -> Result<ModuleCompilationPlan, ModuleError> {
    build_compilation_plan_with_overlays_and_scope_check(program, resolver, overlays, false)
}

fn build_compilation_plan_with_overlays_and_scope_check(
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
    if validate_root_import_bindings {
        validate_import_bindings(program)?;
    }
    collect_root_imports(
        program,
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
        collect_unit(
            &import.path,
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
        };
        if let Some(first) = symbols.get(&key)
            && first.module != module
        {
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExternalContract {
    abi: ForeignAbi,
    unsafe_boundary: bool,
    generic_parameters: Vec<String>,
    parameters: Vec<String>,
    return_type: Option<String>,
}

fn external_contract(declaration: &TopLevelDecl) -> Option<ExternalContract> {
    let TopLevelDecl::ExternalVerb(verb) = declaration else { return None };
    Some(ExternalContract {
        abi: verb.abi,
        unsafe_boundary: verb.unsafe_boundary,
        generic_parameters: verb.generic_parameters.iter().map(generic_parameter_key).collect(),
        parameters: verb.params.iter().map(parameter_key).collect(),
        return_type: verb.return_type.as_ref().map(|return_type| {
            format!("{}:{}", return_access_key(return_type.access), return_type.ty.canonical_key())
        }),
    })
}

fn generic_parameter_key(parameter: &crate::ast::GenericParam) -> String {
    let kind = match &parameter.kind {
        GenericParamKind::Type => "type".to_owned(),
        GenericParamKind::Const { domain } => format!("const:{}", domain.canonical_key()),
    };
    let bound =
        parameter.bound.as_ref().map(crate::ast::TypeName::canonical_key).unwrap_or_default();
    let bounds = parameter
        .bounds
        .iter()
        .map(crate::ast::TypeName::canonical_key)
        .collect::<Vec<_>>()
        .join(",");
    format!("{kind}:{bound}:{bounds}")
}

fn parameter_key(parameter: &crate::ast::Param) -> String {
    format!(
        "{}:{}:{}",
        role_key(&parameter.role),
        dispatch_key(parameter.dispatch),
        parameter.ty.canonical_key()
    )
}

fn role_key(role: &Role) -> &'static str {
    match role {
        Role::Erg => "erg",
        Role::Abs => "abs",
        Role::Dat => "dat",
        Role::Ins => "ins",
    }
}

fn dispatch_key(dispatch: DispatchMode) -> &'static str {
    match dispatch {
        DispatchMode::Static => "static",
        DispatchMode::Dynamic => "dynamic",
    }
}

fn return_access_key(access: ReturnAccess) -> &'static str {
    match access {
        ReturnAccess::Owned => "owned",
        ReturnAccess::Abs => "abs",
    }
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
            collect_unit(
                &import_path,
                unit.implementation(),
                resolver,
                overlays,
                imported_paths,
                units,
                caller_declarations,
                appended_symbols,
                namespace,
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
        let imported = caller_declaration(declaration, namespace);
        if !has_compatible_external_declaration(target, &imported) {
            target.push(imported);
        }
    }
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

fn caller_declaration(declaration: &TopLevelDecl, namespace: Option<&str>) -> TopLevelDecl {
    match declaration {
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
