use std::path::{Path, PathBuf};

use crate::build_graph::invalidate_stale_artifact;
use crate::codegen::NativeEmitError;
use crate::configuration::{CompilerConfiguration, EntryContract};

use super::super::conformance::{ConformanceMode, validate_source_limits};
use super::artifacts::{default_output, write_artifact};
use super::loading::{load_build_program, validate_strict_plan};
use super::options::EmitKind;

pub(crate) fn build_file(
    input: &str,
    output: Option<&Path>,
    emit: EmitKind,
    configuration: &CompilerConfiguration,
) -> i32 {
    build_file_with_mode(input, output, emit, configuration, ConformanceMode::Standard)
}

pub(crate) fn build_file_with_mode(
    input: &str,
    output: Option<&Path>,
    emit: EmitKind,
    configuration: &CompilerConfiguration,
    mode: ConformanceMode,
) -> i32 {
    build_file_with_report(input, output, emit, configuration, true, mode)
}

pub(crate) fn build_file_quiet(
    input: &str,
    output: Option<&Path>,
    emit: EmitKind,
    configuration: &CompilerConfiguration,
) -> i32 {
    build_file_with_report(input, output, emit, configuration, false, ConformanceMode::Standard)
}

fn build_file_with_report(
    input: &str,
    output: Option<&Path>,
    emit: EmitKind,
    configuration: &CompilerConfiguration,
    report_output: bool,
    mode: ConformanceMode,
) -> i32 {
    let Some((source, plan)) = load_build_program(input, configuration) else {
        return 1;
    };
    if !validate_source_limits(Path::new(input), &source, mode) {
        return 1;
    }
    let caller = crate::semantic::filter_program_for_target(plan.caller(), configuration.target());
    if mode.is_strict() && !validate_strict_plan(input, &source, &plan) {
        return 1;
    }
    let Some(fallback_symbol) = first_defined_verb(&caller) else {
        eprintln!("error: `{input}` contains no verb declarations");
        return 1;
    };
    let symbol = configuration
        .entry_symbol()
        .or_else(|| hosted_entry_symbol(configuration, fallback_symbol))
        .unwrap_or(fallback_symbol)
        .to_owned();
    let report_mode = report_output.then_some(mode);
    emit_and_write(input, output, emit, configuration, &plan, &symbol, report_mode)
}

fn emit_and_write(
    input: &str,
    output: Option<&Path>,
    emit: EmitKind,
    configuration: &CompilerConfiguration,
    plan: &crate::modules::ModuleCompilationPlan,
    symbol: &str,
    report_mode: Option<ConformanceMode>,
) -> i32 {
    if let Err(error) = validate_entry(plan.caller(), symbol, emit, configuration.entry_contract())
    {
        eprintln!("error: {error}");
        return 1;
    }
    let output =
        output.map(PathBuf::from).unwrap_or_else(|| default_output(input, emit, configuration));
    if let Err(error) = invalidate_stale_artifact(&output, configuration) {
        eprintln!("error: {error}");
        return 1;
    }
    let objects = match emit_objects(plan, symbol, configuration) {
        Ok(objects) => objects,
        Err(error) => {
            eprintln!("error: cannot build `{input}`: {error}");
            return 1;
        }
    };
    if let Err(error) = write_artifact(input, &output, objects, emit, configuration) {
        eprintln!("error: {error}");
        return 1;
    }
    if let Some(mode) = report_mode {
        if mode.is_strict() {
            println!("built `{}` (strict)", output.display());
        } else {
            println!("built `{}`", output.display());
        }
    }
    0
}

pub(super) struct EmittedObject {
    pub(super) name: String,
    pub(super) bytes: Vec<u8>,
    pub(super) symbols: std::collections::BTreeSet<String>,
}

fn emit_objects(
    plan: &crate::modules::ModuleCompilationPlan,
    symbol: &str,
    configuration: &CompilerConfiguration,
) -> Result<Vec<EmittedObject>, NativeEmitError> {
    let object_plan = plan.object_plan().map_err(|error| NativeEmitError(error.to_string()))?;
    let targeted_caller =
        crate::semantic::filter_program_for_target(plan.caller(), configuration.target());
    let generic_instances = crate::semantic::analyze(&targeted_caller)
        .map_err(|error| NativeEmitError(format!("semantic analysis failed: {error:?}")))?
        .generic_instances;
    let bindings = module_bindings(&object_plan)?;
    let root = &object_plan.units()[0];
    let root_bytes = crate::codegen::emit_program_object_for_target_in_namespace_with_bindings(
        root.program(),
        symbol,
        root.namespace().symbol_prefix(),
        configuration.native_backend(),
        configuration.target(),
        &bindings,
    )?;
    let mut objects = vec![EmittedObject {
        name: "root".to_owned(),
        bytes: root_bytes,
        symbols: declared_symbols(
            root.program(),
            root.namespace().symbol_prefix(),
            symbol,
            &bindings,
        )?,
    }];
    objects.extend(emit_module_objects(
        &object_plan,
        configuration,
        &bindings,
        &generic_instances,
    )?);
    Ok(objects)
}

fn emit_module_objects(
    object_plan: &crate::modules::ModuleObjectPlan,
    configuration: &CompilerConfiguration,
    bindings: &crate::codegen::NativeSymbolBindings,
    generic_instances: &[crate::semantic::GenericInstance],
) -> Result<Vec<EmittedObject>, NativeEmitError> {
    let mut objects = Vec::new();
    for unit in &object_plan.units()[1..] {
        let bytes =
            crate::codegen::emit_module_object_for_target_in_namespace_with_bindings_and_instances(
                unit.program(),
                unit.namespace().symbol_prefix(),
                configuration.native_backend(),
                configuration.target(),
                bindings,
                generic_instances,
            )
            .map_err(|error| {
                NativeEmitError(format!(
                    "module `{}` emission failed: {error}",
                    unit.namespace().module_path()
                ))
            })?;
        objects.push(EmittedObject {
            name: unit.namespace().symbol_prefix().to_owned(),
            bytes,
            symbols: declared_symbols(
                unit.program(),
                unit.namespace().symbol_prefix(),
                "",
                bindings,
            )?,
        });
    }
    Ok(objects)
}

fn declared_symbols(
    program: &crate::ast::Program,
    namespace: &str,
    entry_symbol: &str,
    bindings: &crate::codegen::NativeSymbolBindings,
) -> Result<std::collections::BTreeSet<String>, NativeEmitError> {
    let mut names = Vec::new();
    for declaration in &program.declarations {
        match declaration {
            crate::ast::TopLevelDecl::Verb(verb) => {
                let symbol = if verb.name == entry_symbol {
                    verb.name.clone()
                } else {
                    crate::codegen::SymbolIdentity::new(
                        namespace,
                        crate::codegen::SymbolKind::Verb,
                        &verb.name,
                    )
                    .map_err(|error| NativeEmitError(error.to_string()))?
                    .as_str()
                    .to_owned()
                };
                names.push(symbol);
            }
            crate::ast::TopLevelDecl::ExternalVerb(verb) => {
                names.push(bindings.external_symbol(&verb.name).to_owned());
            }
            _ => {}
        }
    }
    Ok(names.into_iter().collect())
}

fn module_bindings(
    plan: &crate::modules::ModuleObjectPlan,
) -> Result<crate::codegen::NativeSymbolBindings, NativeEmitError> {
    let mut bindings = Vec::new();
    for unit in &plan.units()[1..] {
        for name in unit.exported_verbs() {
            let identity = crate::codegen::SymbolIdentity::new(
                unit.namespace().symbol_prefix(),
                crate::codegen::SymbolKind::Verb,
                name,
            )
            .map_err(|error| NativeEmitError(error.to_string()))?;
            bindings.push((name.clone(), identity.as_str().to_owned()));
        }
    }
    Ok(crate::codegen::NativeSymbolBindings::new(bindings))
}

fn first_defined_verb(program: &crate::ast::Program) -> Option<&str> {
    program.declarations.iter().find_map(|declaration| match declaration {
        crate::ast::TopLevelDecl::Verb(verb) => Some(verb.name.as_str()),
        _ => None,
    })
}

fn validate_entry(
    program: &crate::ast::Program,
    symbol: &str,
    emit: EmitKind,
    contract: EntryContract,
) -> Result<(), String> {
    let Some(crate::ast::TopLevelDecl::Verb(verb)) = program.declarations.iter().find(|decl| {
        matches!(decl, crate::ast::TopLevelDecl::Verb(candidate) if candidate.name == symbol)
    }) else {
        return Err(format!("entry verb `{symbol}` was not found"));
    };
    if matches!(emit, EmitKind::Executable)
        && matches!(contract, EntryContract::Hosted)
        && symbol != "main"
    {
        return Err("hosted executables require entry verb `main`; freestanding targets accept a configured entry symbol".to_owned());
    }
    if matches!(emit, EmitKind::Executable) && !verb.params.is_empty() {
        return Err(format!("executable entry verb `{symbol}` cannot have parameters"));
    }
    Ok(())
}

fn hosted_entry_symbol<'a>(
    configuration: &CompilerConfiguration,
    fallback_symbol: &'a str,
) -> Option<&'a str> {
    matches!(configuration.entry_contract(), EntryContract::Hosted)
        .then_some("main")
        .or(Some(fallback_symbol))
}
