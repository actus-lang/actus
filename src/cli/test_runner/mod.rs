mod collection;
mod execution;
mod output;
mod process;

use crate::configuration::CompilerConfiguration;

use super::conformance::{ConformanceMode, parse_strict_option};

pub(super) fn test_command(arguments: impl Iterator<Item = String>) -> i32 {
    let mode = match parse_test_mode(arguments) {
        Ok(mode) => mode,
        Err(error) => {
            eprintln!("error: {error}");
            return 2;
        }
    };
    let configuration = match load_test_configuration(mode) {
        Ok(configuration) => configuration,
        Err(error) => {
            eprintln!("error: {error}");
            return 1;
        }
    };
    let mut files = Vec::new();
    collection::collect_act_files(&configuration.project_root().join("tests"), &mut files);
    collection::collect_act_files(configuration.source_root(), &mut files);
    files.sort();
    files.dedup();
    let tests = match collection::collect_tests(files, &configuration, mode) {
        Ok(tests) => tests,
        Err(error) => {
            eprintln!("error: {error}");
            return 1;
        }
    };
    execution::run_tests(tests, &configuration, mode)
}

fn parse_test_mode(arguments: impl Iterator<Item = String>) -> Result<ConformanceMode, String> {
    let mut mode = ConformanceMode::Standard;
    for argument in arguments {
        match parse_strict_option(&argument, &mut mode) {
            Ok(true) => continue,
            Ok(false) => return Err(format!("`actus test` does not accept argument `{argument}`")),
            Err(error) => return Err(error),
        }
    }
    Ok(mode)
}

fn load_test_configuration(mode: ConformanceMode) -> Result<CompilerConfiguration, String> {
    if mode.is_strict() {
        CompilerConfiguration::from_current_manifest_strict().map_err(|error| error.to_string())
    } else {
        CompilerConfiguration::from_current_manifest().map_err(|error| error.to_string())
    }
}
