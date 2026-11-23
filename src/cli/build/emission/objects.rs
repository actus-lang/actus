use std::path::{Path, PathBuf};

use crate::build_graph::invalidate_stale_artifact;
use crate::codegen::BackendEmitError;
use crate::configuration::{CompilerConfiguration, EntryContract};

use super::super::super::conformance::{ConformanceMode, validate_source_limits};
use super::super::artifacts::{default_output, write_artifact};
use super::super::loading::{load_build_program, validate_plan};
use super::super::options::EmitKind;

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
    let Some((source, plan, source_map)) = load_build_program(input, configuration) else {
        return 1;
    };
    if mode.is_strict()
        && configuration.native_backend().backend() == crate::codegen::BackendKind::Llvm
        && !validate_llvm_identity()
    {
        return 1;
    }
    if !validate_source_limits(Path::new(input), &source, mode, configuration) {
        return 1;
    }
    let caller = match crate::semantic::filter_program_for_target_checked(
        plan.caller(),
        configuration.target(),
    ) {
        Ok(caller) => caller,
        Err(error) => {
            eprintln!("error: semantic analysis failed: {error:?}");
            return 1;
        }
    };
    if !validate_plan(input, &source, &plan, configuration.target(), &source_map) {
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
    let program = LoadedProgram { plan: &plan, source_map: &source_map };
    emit_and_write(input, output, emit, configuration, program, &symbol, report_mode)
}

/// A resolved build program and the source map its spans point into.
struct LoadedProgram<'a> {
    plan: &'a crate::modules::ModuleCompilationPlan,
    source_map: &'a crate::SharedSourceMap,
}

fn validate_llvm_identity() -> bool {
    #[cfg(feature = "llvm")]
    {
        if let Some(error) = crate::codegen::llvm::strict_identity_error() {
            eprintln!("{error}");
            return false;
        }
    }
    true
}

fn emit_and_write(
    input: &str,
    output: Option<&Path>,
    emit: EmitKind,
    configuration: &CompilerConfiguration,
    program: LoadedProgram<'_>,
    symbol: &str,
    report_mode: Option<ConformanceMode>,
) -> i32 {
    let plan = program.plan;
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
    // A hosted executable runs with the runtime, which reports a failed
    // runtime check with its source location (ADR-0117).
    let hosted_executable = matches!(emit, EmitKind::Executable)
        && matches!(configuration.entry_contract(), EntryContract::Hosted);
    let check_locations = hosted_executable.then_some(program.source_map);
    let object = match emit_object(plan, symbol, configuration, check_locations) {
        Ok(object) => object,
        Err(error) => {
            eprintln!("error: cannot build `{input}`: {error}");
            return 1;
        }
    };
    if let Err(error) = write_artifact(input, &output, object, emit, configuration) {
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

/// Emits the program as one native object (ADR-0113).
pub(crate) fn emit_object(
    plan: &crate::modules::ModuleCompilationPlan,
    symbol: &str,
    configuration: &CompilerConfiguration,
    check_locations: Option<&crate::SharedSourceMap>,
) -> Result<EmittedObject, BackendEmitError> {
    match configuration.native_backend().backend() {
        crate::codegen::BackendKind::Cranelift => {
            emit_object_cranelift(plan, symbol, configuration, check_locations)
        }
        crate::codegen::BackendKind::Llvm => {
            #[cfg(feature = "llvm")]
            {
                super::llvm::emit_object_llvm(plan, symbol, configuration, check_locations)
            }
            #[cfg(not(feature = "llvm"))]
            {
                let _ = (plan, symbol, configuration, check_locations);
                Err(BackendEmitError(
                    "backend `llvm` is not enabled; rebuild with feature `llvm`".to_owned(),
                ))
            }
        }
    }
}

#[cfg(feature = "cranelift")]
fn emit_object_cranelift(
    plan: &crate::modules::ModuleCompilationPlan,
    symbol: &str,
    configuration: &CompilerConfiguration,
    check_locations: Option<&crate::SharedSourceMap>,
) -> Result<EmittedObject, BackendEmitError> {
    let (generic_types, generic_instances) = super::shared::generic_inputs(plan, configuration)?;
    let object_plan = plan
        .object_plan_with_generic_types(&generic_types)
        .map_err(|error| BackendEmitError(error.to_string()))?;
    let bindings =
        module_bindings_with_policy(&object_plan, &generic_instances, plan.has_friend_scope())?;
    super::cranelift::emit_program_object(
        &object_plan,
        symbol,
        configuration,
        &bindings,
        &generic_instances,
        check_locations,
    )
}

#[cfg(not(feature = "cranelift"))]
fn emit_object_cranelift(
    _plan: &crate::modules::ModuleCompilationPlan,
    _symbol: &str,
    _configuration: &CompilerConfiguration,
    _check_locations: Option<&crate::SharedSourceMap>,
) -> Result<EmittedObject, BackendEmitError> {
    Err(BackendEmitError(
        "backend `cranelift` is not enabled; rebuild with feature `cranelift`".to_owned(),
    ))
}

#[cfg(feature = "cranelift")]
fn module_bindings_with_policy(
    plan: &crate::modules::ModuleObjectPlan,
    generic_instances: &[crate::semantic::GenericInstance],
    prefer_first_unique_source: bool,
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
            for instance in
                crate::codegen::generic_instances_of_verb(unit.program(), name, generic_instances)
            {
                let specialized_full = crate::codegen::specialized_generic_name(instance);
                let specialized_name = specialized_full
                    .rsplit("::")
                    .next()
                    .unwrap_or(specialized_full.as_str())
                    .to_owned();
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
    Ok(if prefer_first_unique_source {
        crate::codegen::BackendSymbolBindings::new_prefer_first_unique_source(bindings)
    } else {
        crate::codegen::BackendSymbolBindings::new(bindings)
    })
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
    // The hosted entry returns the process exit status; an aggregate return
    // would be lowered through a return slot the host never passes.
    if matches!(emit, EmitKind::Executable)
        && matches!(contract, EntryContract::Hosted)
        && let Some(return_type) = &verb.return_type
        && !is_exit_status_type(&return_type.ty)
    {
        return Err(format!(
            "hosted entry verb `{symbol}` must return nothing or an integer exit status, \
             found `{}`; return an `Int` and handle errors inside `{symbol}`",
            return_type.ty
        ));
    }
    Ok(())
}

/// `Void`, `Int` or a fixed-width integer of at most 64 bits: the types the
/// host receives directly as the process exit status.
fn is_exit_status_type(type_name: &crate::ast::TypeName) -> bool {
    type_name.arguments.is_empty()
        && (type_name.name == "Int"
            || matches!(
                crate::ast::primitive_type(&type_name.name),
                Some(crate::ast::PrimitiveType::Void)
                    | Some(crate::ast::PrimitiveType::Integer { width: 1..=64, .. })
            ))
}

fn hosted_entry_symbol<'a>(
    configuration: &CompilerConfiguration,
    fallback_symbol: &'a str,
) -> Option<&'a str> {
    matches!(configuration.entry_contract(), EntryContract::Hosted)
        .then_some("main")
        .or(Some(fallback_symbol))
}
