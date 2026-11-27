use crate::cli::terminal_output::{errln, outln};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::diagnostics::Diagnostic;

use super::diagnostics::{report_diagnostics, report_unlocated_diagnostic};

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
    // Strict mode fails the run on every finding, so every finding is an error
    // (ADR-0041: every warning is an error).
    report_diagnostics(
        path,
        source,
        diagnostics.into_iter().map(Diagnostic::promoted_to_error).collect(),
    );
    false
}

/// Every production `.act` file under the package source root, in lexical
/// order: `tests` directories and the `fixtures` and `snapshots` data
/// directories are not production source. This is the one set that strict
/// source limits and strict documentation are checked on, whatever the command
/// and whether or not a file is imported by anything.
pub(super) fn production_source_files(source_root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut pending = vec![source_root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = fs::read_dir(&directory) else { continue };
        for path in entries.flatten().map(|entry| entry.path()) {
            let name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
            if path.is_dir() {
                if !matches!(name, "tests" | "fixtures" | "snapshots") {
                    pending.push(path);
                }
            } else if path.extension().is_some_and(|extension| extension == "act") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// Strict mode checks source limits and public and private-verb documentation
/// on every production file of the package (ADR-0041), plus the explicit entry
/// file when it lies outside that set. Each file is read and checked once and
/// every finding is reported, not only the first. Outside strict mode nothing
/// is checked. A file that does not lex or parse is reported by the frontend,
/// so its documentation is skipped here.
pub(super) fn validate_package_sources(
    entry: Option<&Path>,
    configuration: &crate::configuration::CompilerConfiguration,
    mode: ConformanceMode,
) -> bool {
    if !mode.is_strict() {
        return true;
    }
    let mut files = std::collections::BTreeMap::new();
    for path in production_source_files(configuration.source_root())
        .into_iter()
        .chain(entry.map(Path::to_path_buf))
    {
        let identity = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        files.entry(identity).or_insert(path);
    }
    let mut valid = true;
    for path in files.into_values() {
        if let Ok(source) = fs::read_to_string(&path) {
            valid &= validate_source_limits(&path, &source, mode, configuration);
            valid &= validate_file_documentation(&path, &source, mode);
        }
    }
    valid
}

/// Reports the documentation findings of one source file as errors.
fn validate_file_documentation(path: &Path, source: &str, mode: ConformanceMode) -> bool {
    if !mode.is_strict() {
        return true;
    }
    let (tokens, errors) = crate::lexer::scan(source);
    let Ok(program) = crate::parser::parse(tokens) else { return true };
    if !errors.is_empty() {
        return true;
    }
    let diagnostics = crate::documentation::validate_public_documentation(&program)
        .into_iter()
        // Phase 56 goal 5 redesigns the content contract: the keyword checks
        // E1841 (missing contract part) and E1842 (short or restating summary)
        // are not reliable yet, so strict mode reports only E1840, E1844 and
        // E1845.
        .filter(|issue| {
            issue.code != crate::diagnostics::STRICT_DOCUMENTATION_SECTION
                && issue.code != crate::diagnostics::STRICT_DOCUMENTATION_RESTATEMENT
        })
        .map(|issue| {
            Diagnostic::error(
                issue.code,
                issue.span,
                format!("{}: {}", issue.declaration, issue.message),
            )
            .with_phase(crate::diagnostics::DiagnosticPhase::Conformance)
        })
        .collect::<Vec<_>>();
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
            errln!("error: {error}");
            return 2;
        }
    };
    let paths = match conformance_paths() {
        Ok(paths) => paths,
        Err(error) => {
            errln!("error: {error}");
            return 1;
        }
    };
    if mode.is_strict() {
        let root = match env::current_dir() {
            Ok(root) => root,
            Err(error) => {
                errln!("error: cannot determine repository root: {error}");
                return 1;
            }
        };
        if let Err(error) = crate::conformance::validate_repository_source_exceptions(&root) {
            errln!("error: {error}");
            return 1;
        }
    }
    let configuration = match crate::configuration::CompilerConfiguration::from_current_manifest() {
        Ok(configuration) => configuration,
        Err(error) => {
            errln!("{}", error.report());
            return 1;
        }
    };
    let enforce_limits =
        configuration.source_limit_mode() == crate::configuration::SourceLimitMode::Enabled;
    let scan = scan_conformance_paths(&paths, enforce_limits, mode);
    let (diagnostics_count, approvals_count) = match scan {
        Ok(counts) => counts,
        Err(error) => {
            errln!("error: {error}");
            return 1;
        }
    };
    outln!(
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

/// Reports the findings of every source file. Outside strict mode a finding keeps
/// its own severity and does not fail the run; in strict mode every finding
/// is an error and fails it (ADR-0041).
fn scan_conformance_paths(
    paths: &[PathBuf],
    enforce_limits: bool,
    mode: ConformanceMode,
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
            let diagnostics = if mode.is_strict() {
                report.diagnostics.into_iter().map(Diagnostic::promoted_to_error).collect()
            } else {
                report.diagnostics
            };
            report_diagnostics(path, &source, diagnostics);
        }
    }
    let root =
        env::current_dir().map_err(|error| format!("cannot determine repository root: {error}"))?;
    for diagnostic in crate::conformance::fixture_tree_diagnostics(&root)
        .map_err(|error| format!("cannot scan Actus fixture tree: {error}"))?
    {
        diagnostics_count += 1;
        report_unlocated_diagnostic(&diagnostic);
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
                return Err(super::input::argument_error("conformance", &argument));
            }
            Err(error) => return Err(error),
        }
    }
    Ok(mode)
}

fn source_roots(root: &Path) -> Vec<PathBuf> {
    ["src", "library/std/src", "library/std/tests", "tests"]
        .iter()
        .map(|path| root.join(path))
        .collect()
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
