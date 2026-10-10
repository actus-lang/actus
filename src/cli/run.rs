use std::fs;
use std::path::PathBuf;
use std::process::Command;

use super::build::{EmitKind, build_file_quiet};
use crate::configuration::{BackendKind, BuildProfile, CompilerConfiguration};

struct RunOptions {
    input: Option<String>,
    profile: Option<BuildProfile>,
    backend: Option<BackendKind>,
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
    let configuration = match super::input::configuration_for_input(options.input.as_deref()) {
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
    let configuration = match options.backend {
        Some(backend) => match configuration.with_backend(backend) {
            Ok(configuration) => configuration,
            Err(error) => {
                eprintln!("error: {error}");
                return 1;
            }
        },
        None => configuration,
    };
    let input = super::input::entry_path(&configuration, options.input.as_deref());
    let input = super::input::source_path(&input);
    let output = temporary_output(&configuration);
    if build_file_quiet(&input, Some(&output), EmitKind::Executable, &configuration) != 0 {
        return 1;
    }
    let result = Command::new(&output).args(options.program_arguments).status();
    let _ = fs::remove_file(&output);
    match result {
        Ok(status) => {
            let code = status.code().unwrap_or(1);
            eprintln!("process exited with status {code}");
            code
        }
        Err(error) => {
            eprintln!("error: cannot run `{}`: {error}", output.display());
            1
        }
    }
}

fn parse_run_options(arguments: &mut impl Iterator<Item = String>) -> Result<RunOptions, String> {
    let mut input = None;
    let mut profile = None;
    let mut backend = None;
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
            "--backend" => {
                let name =
                    arguments.next().ok_or_else(|| "missing value after `--backend`".to_owned())?;
                if backend.is_some() {
                    return Err("duplicate backend option".to_owned());
                }
                backend = Some(super::input::parse_backend(&name)?);
            }
            _ if input.is_none() => input = Some(argument),
            _ => return Err(format!("unexpected argument `{argument}`")),
        }
    }
    Ok(RunOptions { input, profile, backend, program_arguments })
}

fn temporary_output(configuration: &CompilerConfiguration) -> PathBuf {
    std::env::temp_dir().join(format!(
        "{}-{}",
        configuration.run_artifact_prefix(),
        std::process::id()
    ))
}
