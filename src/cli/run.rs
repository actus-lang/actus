use std::fs;
use std::path::PathBuf;
use std::process::Command;

use super::build::{EmitKind, build_file};
use crate::configuration::{BuildProfile, CompilerConfiguration};

struct RunOptions {
    input: String,
    profile: Option<BuildProfile>,
    program_arguments: Vec<String>,
}

pub(super) fn run_command(mut arguments: impl Iterator<Item = String>) -> i32 {
    let options = match parse_run_options(&mut arguments) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("error: {error}");
            return 2;
        }
    };
    let configuration =
        match CompilerConfiguration::from_input_path(PathBuf::from(&options.input).as_path()) {
            Ok(configuration) => configuration,
            Err(error) => {
                eprintln!("error: {error}");
                return 1;
            }
        };
    let configuration = match options.profile {
        Some(profile) => configuration.with_profile(profile),
        None => configuration,
    };
    let output = temporary_output(&configuration);
    if build_file(&options.input, Some(&output), EmitKind::Executable, &configuration) != 0 {
        return 1;
    }
    let result = Command::new(&output).args(options.program_arguments).status();
    let _ = fs::remove_file(&output);
    match result {
        Ok(status) => status.code().unwrap_or(1),
        Err(error) => {
            eprintln!("error: cannot run `{}`: {error}", output.display());
            1
        }
    }
}

fn parse_run_options(arguments: &mut impl Iterator<Item = String>) -> Result<RunOptions, String> {
    let mut input = None;
    let mut profile = None;
    let mut program_arguments = Vec::new();
    while let Some(argument) = arguments.next() {
        if argument == "--" {
            program_arguments.extend(arguments);
            break;
        }
        match argument.as_str() {
            "--release" => profile = Some(BuildProfile::Release),
            "--profile" => {
                profile = match arguments.next().as_deref() {
                    Some("debug") => Some(BuildProfile::Debug),
                    Some("release") => Some(BuildProfile::Release),
                    Some(name) => return Err(format!("unsupported build profile `{name}`")),
                    None => return Err("missing value after `--profile`".to_owned()),
                };
            }
            _ if input.is_none() => input = Some(argument),
            _ => return Err(format!("unexpected argument `{argument}`")),
        }
    }
    Ok(RunOptions {
        input: input.unwrap_or_else(|| "src/main.act".to_owned()),
        profile,
        program_arguments,
    })
}

fn temporary_output(configuration: &CompilerConfiguration) -> PathBuf {
    std::env::temp_dir().join(format!(
        "{}-{}",
        configuration.run_artifact_prefix(),
        std::process::id()
    ))
}
