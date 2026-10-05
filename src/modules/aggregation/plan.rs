use std::collections::{HashMap, HashSet};

use crate::ast::{
    DispatchMode, ExternalVerbDecl, ForeignAbi, GenericParamKind, Program, ReturnAccess, Role,
    TopLevelDecl,
};

use super::super::resolver::ModuleResolver;
use super::object_plan::{ModuleObjectPlan, build_object_plan};
use super::types::ModuleError;
use super::unit::ModuleUnit;
use super::validation::export_identity;
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
        build_object_plan(self)
    }

    pub fn implementation(&self) -> Program {
        let mut declarations = self.local.declarations.clone();
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
    let mut caller_declarations = Vec::new();
    let mut local_declarations = Vec::new();
    let mut units = Vec::new();
    let mut imported_paths = HashSet::new();
    for declaration in &program.declarations {
        let TopLevelDecl::Import(import) = declaration else {
            local_declarations.push(declaration.clone());
            caller_declarations.push(declaration.clone());
            continue;
        };
        if !imported_paths.insert(import.path.clone()) {
            continue;
        }
        collect_unit(
            &import.path,
            program,
            resolver,
            overlays,
            &mut imported_paths,
            &mut units,
            &mut caller_declarations,
        )?;
    }
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
) -> Result<(), ModuleError> {
    let unit = super::unit::load_module_unit_with_overlays(resolver, module_path, overlays)?;
    validate_import_visibility(importer, module_path, &unit)?;
    for declaration in &unit.implementation().declarations {
        let TopLevelDecl::Import(import) = declaration else { continue };
        let import_path = qualify_import_path(module_path, &import.path);
        if imported_paths.insert(import_path.clone()) {
            collect_unit(
                &import_path,
                unit.implementation(),
                resolver,
                overlays,
                imported_paths,
                units,
                caller_declarations,
            )?;
        }
    }
    append_public_declarations(caller_declarations, &unit);
    units.push(unit);
    Ok(())
}

pub(super) fn qualify_import_path(importer: &str, imported: &str) -> String {
    if importer.starts_with("std::") && !imported.contains("::") {
        format!("std::{imported}")
    } else {
        imported.to_owned()
    }
}

fn append_public_declarations(target: &mut Vec<TopLevelDecl>, unit: &ModuleUnit) {
    target.extend(
        unit.implementation()
            .declarations
            .iter()
            .filter(|declaration| {
                let Some((kind, name)) = export_identity(declaration) else { return false };
                unit.exports().contains(kind, &name)
            })
            .map(caller_declaration),
    );
}

fn caller_declaration(declaration: &TopLevelDecl) -> TopLevelDecl {
    match declaration {
        TopLevelDecl::Verb(verb) => TopLevelDecl::ExternalVerb(ExternalVerbDecl {
            is_open: verb.is_open,
            doc: verb.doc.clone(),
            contract: verb.contract.clone(),
            unsafe_boundary: false,
            module_import: true,
            abi: ForeignAbi::C,
            metadata: verb.metadata.clone(),
            name: verb.name.clone(),
            generic_parameters: verb.generic_parameters.clone(),
            params: verb.params.clone(),
            return_type: verb.return_type.clone(),
            span: verb.span,
        }),
        _ => declaration.clone(),
    }
}
