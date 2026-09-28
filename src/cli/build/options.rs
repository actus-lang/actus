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
    let input_configuration = match configured_build(&options) {
        Ok(configuration) => configuration,
        Err(error) => {
            eprintln!("error: {error}");
            return 1;
        }
    };
    let input = build_input(&options, &input_configuration);
    build_file_with_mode(
        &input,
        options.output.as_deref().map(Path::new),
        options.emit,
        &input_configuration,
        options.mode,
    )
}

fn configured_build(options: &BuildOptions) -> Result<CompilerConfiguration, String> {
    let configuration = configuration_for_build(options.input.as_deref(), options.mode)?;
    Ok(options.profile.map_or_else(
        || configuration.clone(),
        |profile| configuration.clone().with_profile(profile),
    ))
}

fn build_input(options: &BuildOptions, configuration: &CompilerConfiguration) -> String {
    options
        .input
        .clone()
        .unwrap_or_else(|| configuration.default_entry_path().display().to_string())
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
    let mut options = BuildOptions {
        input: None,
        output: None,
        emit: EmitKind::Object,
        profile: None,
        mode: ConformanceMode::Standard,
    };
    while let Some(argument) = arguments.next() {
        parse_build_argument(argument, &mut arguments, &mut options)?;
    }
    Ok(options)
}

fn parse_build_argument(
    argument: String,
    arguments: &mut impl Iterator<Item = String>,
    options: &mut BuildOptions,
) -> Result<(), String> {
    match argument.as_str() {
        "--strict" => parse_strict_option(&argument, &mut options.mode).map(|_| ())?,
        "--release" => set_profile(options, BuildProfile::Release)?,
        "--profile" => set_profile(options, parse_profile(arguments)?)?,
        "-o" => set_output(options, arguments.next())?,
        "--emit" => options.emit = parse_emit_kind(arguments.next())?,
        _ if options.input.is_none() && !argument.starts_with('-') => {
            options.input = Some(argument)
        }
        _ => return Err(format!("unexpected build argument `{argument}`")),
    }
    Ok(())
}

fn set_profile(options: &mut BuildOptions, profile: BuildProfile) -> Result<(), String> {
    if options.profile.replace(profile).is_some() {
        return Err("duplicate or conflicting profile option".to_owned());
    }
    Ok(())
}

fn parse_profile(arguments: &mut impl Iterator<Item = String>) -> Result<BuildProfile, String> {
    let Some(name) = arguments.next() else {
        return Err("missing value after `--profile`".to_owned());
    };
    match name.as_str() {
        "debug" => Ok(BuildProfile::Debug),
        "release" => Ok(BuildProfile::Release),
        _ => Err(format!("unsupported build profile `{name}`")),
    }
}

fn set_output(options: &mut BuildOptions, output: Option<String>) -> Result<(), String> {
    if options.output.is_some() {
        return Err("duplicate output option".to_owned());
    }
    options.output = output;
    options.output.as_ref().map(|_| ()).ok_or_else(|| "missing output path after `-o`".to_owned())
}

fn parse_emit_kind(kind: Option<String>) -> Result<EmitKind, String> {
    let Some(kind) = kind else {
        return Err("missing value after `--emit`".to_owned());
    };
    match kind.as_str() {
        "obj" => Ok(EmitKind::Object),
        "exe" => Ok(EmitKind::Executable),
        _ => Err(format!("unsupported emission kind `{kind}`")),
    }
}
