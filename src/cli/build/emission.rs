use std::path::{Path, PathBuf};

use crate::build_graph::invalidate_stale_artifact;
use crate::codegen::NativeEmitError;
use crate::configuration::{CompilerConfiguration, EntryContract};

use super::super::conformance::{ConformanceMode, validate_source_limits};
use super::artifacts::{default_output, write_artifact};
use super::loading::{load_build_program, validate_strict_program};
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
    let Some((source, program)) = load_build_program(input, configuration) else {
        return 1;
    };
    if !validate_source_limits(Path::new(input), &source, mode) {
        return 1;
    }
    let program = crate::semantic::filter_program_for_target(&program, configuration.target());
    if mode.is_strict() && !validate_strict_program(input, &source, &program) {
        return 1;
    }
    let Some(fallback_symbol) = first_defined_verb(&program) else {
        eprintln!("error: `{input}` contains no verb declarations");
        return 1;
    };
    let symbol = configuration
        .entry_symbol()
        .or_else(|| hosted_entry_symbol(configuration, fallback_symbol))
        .unwrap_or(fallback_symbol)
        .to_owned();
    let report_mode = report_output.then_some(mode);
    emit_and_write(input, output, emit, configuration, &program, &symbol, report_mode)
}

fn emit_and_write(
    input: &str,
    output: Option<&Path>,
    emit: EmitKind,
    configuration: &CompilerConfiguration,
    program: &crate::ast::Program,
    symbol: &str,
    report_mode: Option<ConformanceMode>,
) -> i32 {
    if let Err(error) = validate_entry(program, symbol, emit, configuration.entry_contract()) {
        eprintln!("error: {error}");
        return 1;
    }
    let output =
        output.map(PathBuf::from).unwrap_or_else(|| default_output(input, emit, configuration));
    if let Err(error) = invalidate_stale_artifact(&output, configuration) {
        eprintln!("error: {error}");
        return 1;
    }
    let bytes = match emit_object(program, symbol, configuration) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("error: cannot build `{input}`: {error}");
            return 1;
        }
    };
    if let Err(error) = write_artifact(input, &output, bytes, emit, configuration) {
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

fn emit_object(
    program: &crate::ast::Program,
    symbol: &str,
    configuration: &CompilerConfiguration,
) -> Result<Vec<u8>, NativeEmitError> {
    crate::codegen::emit_program_object_for_target(
        program,
        symbol,
        configuration.native_backend(),
        configuration.target(),
    )
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
