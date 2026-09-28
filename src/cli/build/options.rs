use std::path::Path;

use crate::configuration::{BuildProfile, CompilerConfiguration};

use super::super::conformance::{ConformanceMode, parse_strict_option};
use super::emission::build_file_with_mode;

#[derive(Clone, Copy)]
pub(crate) enum EmitKind {
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

pub(crate) fn build_command(arguments: impl Iterator<Item = String>) -> i32 {
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
