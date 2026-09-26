use std::fs;
use std::path::{Path, PathBuf};

use crate::build_graph::{invalidate_stale_artifact, write_metadata};
use crate::codegen::link_object;
use crate::configuration::{BuildProfile, CompilerConfiguration, EntryContract};
use crate::diagnostics::{render_lex_error, render_parse_error};
use crate::lexer::scan;
use crate::modules::{ModuleResolver, resolve_imports};
use crate::parser::parse;

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
}

pub(super) fn build_command(arguments: impl Iterator<Item = String>) -> i32 {
    let options = match parse_build_options(arguments) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("error: {error}");
            return 2;
        }
    };
    let input_configuration = match configuration_for_build(options.input.as_deref()) {
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
    build_file(&input, options.output.as_deref().map(Path::new), options.emit, &input_configuration)
}

fn configuration_for_build(input: Option<&str>) -> Result<CompilerConfiguration, String> {
    let configuration = match input {
        Some(path) => CompilerConfiguration::from_input_path(Path::new(path)),
        None => {
            let current = std::env::current_dir()
                .map_err(|error| format!("cannot read current directory: {error}"))?;
            CompilerConfiguration::from_input_path(&current)
        }
    }
    .map_err(|error| error.to_string())?;
    if input.is_none() && !configuration.has_manifest() {
        return Err("cannot discover Arca.toml from the current directory".to_owned());
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
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
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
    Ok(BuildOptions { input, output, emit, profile })
}

pub(super) fn build_file(
    input: &str,
    output: Option<&Path>,
    emit: EmitKind,
    configuration: &CompilerConfiguration,
) -> i32 {
    build_file_with_report(input, output, emit, configuration, true)
}

pub(super) fn build_file_quiet(
    input: &str,
    output: Option<&Path>,
    emit: EmitKind,
    configuration: &CompilerConfiguration,
) -> i32 {
    build_file_with_report(input, output, emit, configuration, false)
}

fn build_file_with_report(
    input: &str,
    output: Option<&Path>,
    emit: EmitKind,
    configuration: &CompilerConfiguration,
    report_output: bool,
) -> i32 {
    let source = match fs::read_to_string(input) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("error: cannot read `{input}`: {error}");
            return 1;
        }
    };
    let (tokens, lex_errors) = scan(&source);
    if !lex_errors.is_empty() {
        for error in &lex_errors {
            eprintln!("{input}: {}", render_lex_error(&source, error));
        }
        return 1;
    }
    let program = match parse(tokens) {
        Ok(program) => program,
        Err(error) => {
            eprintln!("{input}: {}", render_parse_error(&source, &error));
            return 1;
        }
    };
    let program = match resolve_imports(
        &program,
        &ModuleResolver::with_dependencies(
            configuration.source_root(),
            configuration.dependency_roots(),
        ),
    ) {
        Ok(program) => program,
        Err(error) => {
            eprintln!("error: cannot resolve imports for `{input}`: {error}");
            return 1;
        }
    };
    let Some(fallback_symbol) = first_defined_verb(&program) else {
        eprintln!("error: `{input}` contains no verb declarations");
        return 1;
    };
    let symbol = configuration
        .entry_symbol()
        .or_else(|| hosted_entry_symbol(configuration, fallback_symbol))
        .unwrap_or(fallback_symbol)
        .to_owned();
    emit_and_write(input, output, emit, configuration, &program, &symbol, report_output)
}

fn emit_and_write(
    input: &str,
    output: Option<&Path>,
    emit: EmitKind,
    configuration: &CompilerConfiguration,
    program: &crate::ast::Program,
    symbol: &str,
    report_output: bool,
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
    if report_output {
        println!("built `{}`", output.display());
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
