mod collection;
mod execution;
mod output;
mod process;

use crate::configuration::CompilerConfiguration;

use super::conformance::{ConformanceMode, parse_strict_option};

pub(super) struct TestOptions {
    pub(super) mode: ConformanceMode,
    pub(super) filter: Option<String>,
    pub(super) module: Option<String>,
    pub(super) file: Option<String>,
    pub(super) exact: bool,
    pub(super) list: bool,
    pub(super) mirror: bool,
}

pub(super) fn test_command(arguments: impl Iterator<Item = String>) -> i32 {
    let options = match parse_test_options(arguments) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("error: {error}");
            return 2;
        }
    };
    let configuration = match load_test_configuration(options.mode) {
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
    let tests = match collection::collect_tests(files, &configuration, &options) {
        Ok(tests) => tests,
        Err(error) => {
            eprintln!("error: {error}");
            return 1;
        }
    };
    execution::run_tests(tests, &configuration, &options)
}

fn parse_test_options(mut arguments: impl Iterator<Item = String>) -> Result<TestOptions, String> {
    let mut mode = ConformanceMode::Standard;
    let mut filter = None;
    let mut module = None;
    let mut file = None;
    let mut exact = false;
    let mut list = false;
    let mut mirror = false;
    while let Some(argument) = arguments.next() {
        if argument == "--exact" {
            exact = true;
        } else if argument == "--list" {
            list = true;
        } else if argument == "--mirror" {
            mirror = true;
        } else if argument == "--module" {
            module = Some(next_test_value(&mut arguments, "--module")?);
        } else if let Some(value) = argument.strip_prefix("--module=") {
            module = Some(non_empty_test_value(value, "--module")?);
        } else if argument == "--file" {
            file = Some(next_test_value(&mut arguments, "--file")?);
        } else if let Some(value) = argument.strip_prefix("--file=") {
            file = Some(non_empty_test_value(value, "--file")?);
        } else if argument.starts_with('-') {
            match parse_strict_option(&argument, &mut mode) {
                Ok(true) => {}
                Ok(false) => {
                    return Err(format!("`actus test` does not accept argument `{argument}`"));
                }
                Err(error) => return Err(error),
            }
        } else if filter.replace(argument).is_some() {
            return Err("`actus test` accepts only one positional filter".to_owned());
        }
    }
    Ok(TestOptions { mode, filter, module, file, exact, list, mirror })
}

fn next_test_value(
    arguments: &mut impl Iterator<Item = String>,
    option: &str,
) -> Result<String, String> {
    let value = arguments.next().ok_or_else(|| format!("missing value for `{option}`"))?;
    non_empty_test_value(&value, option)
}

fn non_empty_test_value(value: &str, option: &str) -> Result<String, String> {
    if value.is_empty() { Err(format!("empty value for `{option}`")) } else { Ok(value.to_owned()) }
}

fn load_test_configuration(mode: ConformanceMode) -> Result<CompilerConfiguration, String> {
    if mode.is_strict() {
        CompilerConfiguration::from_current_manifest_strict().map_err(|error| error.to_string())
    } else {
        CompilerConfiguration::from_current_manifest().map_err(|error| error.to_string())
    }
}
