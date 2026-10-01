use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use super::diagnostics::report_diagnostics;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ConformanceMode {
    Standard,
    Strict,
}

pub(super) fn validate_source_limits(
    path: &Path,
    source: &str,
    mode: ConformanceMode,
    configuration: &crate::configuration::CompilerConfiguration,
) -> bool {
    if !mode.is_strict() {
        return true;
    }
    let diagnostics = crate::conformance::inspect_source_with_enforcement(
        path,
        source,
        configuration.source_limit_mode() == crate::configuration::SourceLimitMode::Enabled,
    );
    if diagnostics.is_empty() {
        return true;
    }
    report_diagnostics(path, source, diagnostics);
    false
}

pub(super) fn conformance_command(arguments: impl Iterator<Item = String>) -> i32 {
    let mode = match parse_conformance_mode(arguments) {
        Ok(mode) => mode,
        Err(error) => {
            eprintln!("error: {error}");
            return 2;
        }
    };
    let paths = match conformance_paths() {
        Ok(paths) => paths,
        Err(error) => {
            eprintln!("error: {error}");
            return 1;
        }
    };
    if mode.is_strict() {
        let root = match env::current_dir() {
            Ok(root) => root,
            Err(error) => {
                eprintln!("error: cannot determine repository root: {error}");
                return 1;
            }
        };
        if let Err(error) = crate::conformance::validate_repository_source_exceptions(&root) {
            eprintln!("error: {error}");
            return 1;
        }
    }
    let configuration = match crate::configuration::CompilerConfiguration::from_current_manifest() {
        Ok(configuration) => configuration,
        Err(error) => {
            eprintln!("error: {error}");
            return 1;
        }
    };
    let enforce_limits =
        configuration.source_limit_mode() == crate::configuration::SourceLimitMode::Enabled;
    let (diagnostics_count, approvals_count) = match scan_conformance_paths(&paths, enforce_limits)
    {
        Ok(counts) => counts,
        Err(error) => {
            eprintln!("error: {error}");
            return 1;
        }
    };
    println!(
        "conformance: scanned {} source files; {} diagnostics; {} accepted limitless scopes{}",
        paths.len(),
        diagnostics_count,
        approvals_count,
        if mode.is_strict() { " (strict)" } else { "" }
    );
    i32::from(mode.is_strict() && diagnostics_count != 0)
}

fn conformance_paths() -> Result<Vec<PathBuf>, String> {
    let root =
        env::current_dir().map_err(|error| format!("cannot determine repository root: {error}"))?;
    let roots = source_roots(&root);
    let root_refs = roots.iter().map(PathBuf::as_path).collect::<Vec<_>>();
    crate::conformance::source_paths(&root_refs)
        .map_err(|error| format!("cannot scan source roots: {error}"))
}

fn scan_conformance_paths(
    paths: &[PathBuf],
    enforce_limits: bool,
) -> Result<(usize, usize), String> {
    let mut diagnostics_count = 0;
    let mut approvals_count = 0;
    for path in paths {
        let source = fs::read_to_string(path)
            .map_err(|error| format!("cannot read `{}`: {error}", path.display()))?;
        let report = crate::conformance::inspect_source_report_with_enforcement(
            path,
            &source,
            enforce_limits,
        );
        diagnostics_count += report.diagnostics.len();
        approvals_count += report.approvals.len();
        if !report.diagnostics.is_empty() {
            report_diagnostics(path, &source, report.diagnostics);
        }
    }
    let root =
        env::current_dir().map_err(|error| format!("cannot determine repository root: {error}"))?;
    for diagnostic in crate::conformance::fixture_tree_diagnostics(&root)
        .map_err(|error| format!("cannot scan Actus fixture tree: {error}"))?
    {
        diagnostics_count += 1;
        let source_path =
            diagnostic.source_path().map(str::to_owned).unwrap_or_else(|| "<unknown>".to_owned());
        report_diagnostics(Path::new(&source_path), "", vec![diagnostic]);
    }
    Ok((diagnostics_count, approvals_count))
}

fn parse_conformance_mode(
    arguments: impl Iterator<Item = String>,
) -> Result<ConformanceMode, String> {
    let mut mode = ConformanceMode::Standard;
    for argument in arguments {
        match parse_strict_option(&argument, &mut mode) {
            Ok(true) => continue,
            Ok(false) => {
                return Err(format!("`actus conformance` does not accept argument `{argument}`"));
            }
            Err(error) => return Err(error),
        }
    }
    Ok(mode)
}

fn source_roots(root: &Path) -> Vec<PathBuf> {
    ["src", "library/std/src", "tests"].iter().map(|path| root.join(path)).collect()
}

impl ConformanceMode {
    pub(super) const fn is_strict(self) -> bool {
        matches!(self, Self::Strict)
    }
}

pub(super) fn parse_strict_option(
    argument: &str,
    mode: &mut ConformanceMode,
) -> Result<bool, String> {
    if argument != "--strict" {
        return Ok(false);
    }
    if mode.is_strict() {
        return Err("duplicate `--strict` option".to_owned());
    }
    *mode = ConformanceMode::Strict;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::{ConformanceMode, parse_strict_option};

    #[test]
    fn strict_option_is_explicit_and_non_repeatable() {
        let mut mode = ConformanceMode::Standard;
        assert!(parse_strict_option("--strict", &mut mode).expect("strict option should parse"));
        assert!(mode.is_strict());
        assert_eq!(
            parse_strict_option("--strict", &mut mode),
            Err("duplicate `--strict` option".to_owned())
        );
    }

    #[test]
    fn unrelated_arguments_are_left_to_command_parsers() {
        let mut mode = ConformanceMode::Standard;
        assert!(!parse_strict_option("--release", &mut mode).expect("unrelated option"));
        assert_eq!(mode, ConformanceMode::Standard);
    }
}
