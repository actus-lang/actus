use std::fs;
use std::path::{Path, PathBuf};

use crate::build_graph::{invalidate_stale_artifact, write_metadata};
use crate::codegen::link_object;
use crate::configuration::{BuildProfile, CompilerConfiguration, EntryContract};
use crate::diagnostics::{lex_diagnostic, parse_diagnostic, semantic_diagnostic};
use crate::lexer::scan;
use crate::modules::{ModuleResolver, resolve_imports};
use crate::parser::parse;
use crate::semantic::filter_program_for_target;

use super::conformance::{ConformanceMode, parse_strict_option};
use super::diagnostics::{report_diagnostic, report_diagnostics};

#[derive(Clone, Copy)]
pub(super) enum EmitKind {
    Object,
    Executable,
}

struct BuildOptions {
    input: Option<String>,
    output: Option<String>,
    emit: EmitKind,
    profile: Option<BuildProfile>,
    mode: ConformanceMode,
}

pub(super) fn build_command(arguments: impl Iterator<Item = String>) -> i32 {
    let options = match parse_build_options(arguments) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("error: {error}");
            return 2;
        }
    };
    let input_configuration = match configuration_for_build(options.input.as_deref(), options.mode)
    {
        Ok(configuration) => configuration,
        Err(error) => {
            eprintln!("error: {error}");
            return 1;
        }
    };
    let input_configuration = match options.profile {
        Some(profile) => input_configuration.with_profile(profile),
        None => input_configuration,
    };
    let input = options
        .input
        .unwrap_or_else(|| input_configuration.default_entry_path().display().to_string());
    build_file_with_mode(
        &input,
        options.output.as_deref().map(Path::new),
        options.emit,
        &input_configuration,
        options.mode,
    )
}

fn configuration_for_build(
    input: Option<&str>,
    mode: ConformanceMode,
) -> Result<CompilerConfiguration, String> {
    let configuration = match input {
        Some(path) if mode.is_strict() => {
            CompilerConfiguration::from_input_path_strict(Path::new(path))
                .map_err(|error| error.to_string())
        }
        Some(path) => CompilerConfiguration::from_input_path(Path::new(path))
            .map_err(|error| error.to_string()),
        None => {
            let current = std::env::current_dir()
                .map_err(|error| format!("cannot read current directory: {error}"))?;
            if mode.is_strict() {
                CompilerConfiguration::from_input_path_strict(&current)
                    .map_err(|error| error.to_string())
            } else {
                CompilerConfiguration::from_input_path(&current).map_err(|error| error.to_string())
            }
        }
    }?;
    if input.is_none() && !configuration.has_manifest() {
        return Err("cannot discover Actus.toml from the current directory".to_owned());
    }
    Ok(configuration)
}

fn parse_build_options(
    mut arguments: impl Iterator<Item = String>,
) -> Result<BuildOptions, String> {
    let mut input = None;
    let mut output = None;
    let mut emit = EmitKind::Object;
    let mut profile = None;
    let mut mode = ConformanceMode::Standard;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--strict" => {
                parse_strict_option(&argument, &mut mode)?;
            }
            "--release" => {
                if profile.replace(BuildProfile::Release).is_some() {
                    return Err("duplicate or conflicting profile option".to_owned());
                }
            }
            "--profile" => {
                let Some(name) = arguments.next() else {
                    return Err("missing value after `--profile`".to_owned());
                };
                let parsed_profile = match name.as_str() {
                    "debug" => BuildProfile::Debug,
                    "release" => BuildProfile::Release,
                    _ => return Err(format!("unsupported build profile `{name}`")),
                };
                if profile.replace(parsed_profile).is_some() {
                    return Err("duplicate or conflicting profile option".to_owned());
                }
            }
            "-o" => {
                if output.is_some() {
                    return Err("duplicate output option".to_owned());
                }
                output = arguments.next();
                if output.is_none() {
                    return Err("missing output path after `-o`".to_owned());
                }
            }
            "--emit" => {
                let Some(kind) = arguments.next() else {
                    return Err("missing value after `--emit`".to_owned());
                };
                emit = match kind.as_str() {
                    "obj" => EmitKind::Object,
                    "exe" => EmitKind::Executable,
                    _ => return Err(format!("unsupported emission kind `{kind}`")),
                };
            }
            _ if input.is_none() && !argument.starts_with('-') => input = Some(argument),
            _ => return Err(format!("unexpected build argument `{argument}`")),
        }
    }
    Ok(BuildOptions { input, output, emit, profile, mode })
}

pub(super) fn build_file(
    input: &str,
    output: Option<&Path>,
    emit: EmitKind,
    configuration: &CompilerConfiguration,
) -> i32 {
    build_file_with_mode(input, output, emit, configuration, ConformanceMode::Standard)
}

pub(super) fn build_file_with_mode(
    input: &str,
    output: Option<&Path>,
    emit: EmitKind,
    configuration: &CompilerConfiguration,
    mode: ConformanceMode,
) -> i32 {
    build_file_with_report(input, output, emit, configuration, true, mode)
}

pub(super) fn build_file_quiet(
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
    let program = filter_program_for_target(&program, configuration.target());
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

fn load_build_program(
    input: &str,
    configuration: &CompilerConfiguration,
) -> Option<(String, crate::ast::Program)> {
    let source = match fs::read_to_string(input) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("error: cannot read `{input}`: {error}");
            return None;
        }
    };
    let (tokens, lex_errors) = scan(&source);
    if !lex_errors.is_empty() {
        report_diagnostics(
            Path::new(input),
            &source,
            lex_errors.iter().map(lex_diagnostic).collect(),
        );
        return None;
    }
    let program = match parse(tokens) {
        Ok(program) => program,
        Err(error) => {
            report_diagnostic(Path::new(input), &source, parse_diagnostic(&error));
            return None;
        }
    };
    let resolver = ModuleResolver::with_dependencies(
        configuration.source_root(),
        configuration.dependency_roots(),
    );
    match resolve_imports(&program, &resolver) {
        Ok(program) => Some((source, program)),
        Err(error) => {
            eprintln!("error: cannot resolve imports for `{input}`: {error}");
            None
        }
    }
}

fn validate_strict_program(input: &str, source: &str, program: &crate::ast::Program) -> bool {
    match crate::semantic::analyze(program) {
        Ok(_) => true,
        Err(error) => {
            report_diagnostic(Path::new(input), source, semantic_diagnostic(&error));
            false
        }
    }
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
) -> Result<Vec<u8>, crate::codegen::NativeEmitError> {
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
        crate::ast::TopLevelDecl::ExternalVerb(_)
        | crate::ast::TopLevelDecl::Struct(_)
        | crate::ast::TopLevelDecl::Pack(_)
        | crate::ast::TopLevelDecl::Enum(_)
        | crate::ast::TopLevelDecl::Role(_)
        | crate::ast::TopLevelDecl::Perform(_)
        | crate::ast::TopLevelDecl::OpenSibling(_)
        | crate::ast::TopLevelDecl::Import(_) => None,
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
        return Err(
            "hosted executables require entry verb `main`; freestanding targets accept a configured entry symbol"
                .to_owned(),
        );
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

fn write_artifact(
    input: &str,
    output: &Path,
    bytes: Vec<u8>,
    emit: EmitKind,
    configuration: &CompilerConfiguration,
) -> Result<(), String> {
    let persistent_capsula_artifact = output.starts_with(configuration.capsula_target_directory());
    let object = matches!(emit, EmitKind::Executable)
        .then(|| output.with_extension(if persistent_capsula_artifact { "obj" } else { "o" }));
    let object_path = object.as_deref().unwrap_or(output);
    if let Some(parent) = object_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create `{}`: {error}", parent.display()))?;
    }
    fs::write(object_path, bytes)
        .map_err(|error| format!("cannot write `{}`: {error}", object_path.display()))?;
    if let Some(object_path) = object {
        let result = link_object(&object_path, output, configuration)
            .map_err(|error| format!("cannot link `{input}`: {error}"));
        if !persistent_capsula_artifact {
            let _ = fs::remove_file(object_path);
        }
        result
            .and_then(|()| write_metadata(output, configuration).map_err(|error| error.to_string()))
    } else {
        write_metadata(output, configuration).map_err(|error| error.to_string())
    }
}

fn default_output(input: &str, emit: EmitKind, configuration: &CompilerConfiguration) -> PathBuf {
    let path = Path::new(input);
    let extension = if matches!(emit, EmitKind::Object) { "obj" } else { "bin" };
    configuration
        .capsula_target_directory()
        .join(path.file_stem().unwrap_or_default())
        .with_extension(extension)
}
