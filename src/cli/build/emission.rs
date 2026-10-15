use std::path::{Path, PathBuf};

use crate::build_graph::invalidate_stale_artifact;
use crate::codegen::BackendEmitError;
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
    if !validate_source_limits(Path::new(input), &source, mode, configuration) {
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
        if configuration.native_backend().verifies_no_float_ir() {
            println!("verified generated native IR: no floating-point instructions");
        }
    }
    0
}

pub(crate) struct EmittedObject {
    pub(crate) name: String,
    pub(crate) bytes: Vec<u8>,
    pub(crate) symbols: std::collections::BTreeSet<String>,
}

pub(crate) fn emit_objects(
    plan: &crate::modules::ModuleCompilationPlan,
    symbol: &str,
    configuration: &CompilerConfiguration,
) -> Result<Vec<EmittedObject>, BackendEmitError> {
    match configuration.native_backend().backend() {
        crate::codegen::BackendKind::Cranelift => {
            emit_objects_cranelift(plan, symbol, configuration)
        }
        crate::codegen::BackendKind::Llvm => {
            #[cfg(feature = "llvm")]
            {
                super::llvm_emission::emit_objects_llvm(plan, symbol, configuration)
            }
            #[cfg(not(feature = "llvm"))]
            {
                let _ = (plan, symbol, configuration);
                Err(BackendEmitError(
                    "backend `llvm` is not enabled; rebuild with feature `llvm`".to_owned(),
                ))
            }
        }
    }
}

#[cfg(feature = "cranelift")]
fn emit_objects_cranelift(
    plan: &crate::modules::ModuleCompilationPlan,
    symbol: &str,
    configuration: &CompilerConfiguration,
) -> Result<Vec<EmittedObject>, BackendEmitError> {
    let (generic_types, generic_instances) =
        super::shared_emission::generic_inputs(plan, configuration)?;
    let object_plan = plan
        .object_plan_with_generic_types(&generic_types)
        .map_err(|error| BackendEmitError(error.to_string()))?;
    let bindings = module_bindings(&object_plan, &generic_instances)?;
    emit_shared_cranelift(&object_plan, symbol, configuration, &bindings, &generic_instances)
}

#[cfg(feature = "cranelift")]
fn emit_shared_cranelift(
    object_plan: &crate::modules::ModuleObjectPlan,
    symbol: &str,
    configuration: &CompilerConfiguration,
    bindings: &crate::codegen::BackendSymbolBindings,
    generic_instances: &[crate::semantic::GenericInstance],
) -> Result<Vec<EmittedObject>, BackendEmitError> {
    if generic_instances.is_empty() {
        return super::cranelift_shared::emit_shared_scalar_units(
            object_plan,
            symbol,
            configuration,
            bindings,
        );
    }
    super::cranelift_shared::emit_shared_specialized_units(
        object_plan,
        symbol,
        configuration,
        bindings,
        generic_instances,
    )
}

#[cfg(not(feature = "cranelift"))]
fn emit_objects_cranelift(
    _plan: &crate::modules::ModuleCompilationPlan,
    _symbol: &str,
    _configuration: &CompilerConfiguration,
) -> Result<Vec<EmittedObject>, BackendEmitError> {
    Err(BackendEmitError(
        "backend `cranelift` is not enabled; rebuild with feature `cranelift`".to_owned(),
    ))
}

#[cfg(feature = "cranelift")]
fn module_bindings(
    plan: &crate::modules::ModuleObjectPlan,
    generic_instances: &[crate::semantic::GenericInstance],
) -> Result<crate::codegen::BackendSymbolBindings, BackendEmitError> {
    let mut bindings = Vec::new();
    for unit in &plan.units()[1..] {
        for name in unit.exported_verbs() {
            let identity = crate::codegen::SymbolIdentity::new(
                unit.namespace().symbol_prefix(),
                crate::codegen::SymbolKind::Verb,
                name,
            )
            .map_err(|error| BackendEmitError(error.to_string()))?;
            bindings.push((
                unit.namespace().symbol_prefix().to_owned(),
                name.clone(),
                identity.as_str().to_owned(),
            ));
            for instance in generic_instances.iter().filter(|instance| instance.name == *name) {
                let specialized_name = crate::codegen::specialized_generic_name(instance);
                let specialized_identity = crate::codegen::SymbolIdentity::new(
                    unit.namespace().symbol_prefix(),
                    crate::codegen::SymbolKind::Verb,
                    &specialized_name,
                )
                .map_err(|error| BackendEmitError(error.to_string()))?;
                bindings.push((
                    unit.namespace().symbol_prefix().to_owned(),
                    specialized_name,
                    specialized_identity.as_str().to_owned(),
                ));
            }
        }
    }
    Ok(crate::codegen::BackendSymbolBindings::new(bindings))
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
