use crate::cli::terminal_output::errln;
use std::io::{self, IsTerminal};
use std::path::Path;

use crate::SharedSourceMap;
use crate::diagnostics::{
    Diagnostic, lex_diagnostic, parse_diagnostic, render_colored_diagnostic,
    render_unlocated_diagnostic, sort_diagnostics,
};
use crate::session::FrontendError;

pub(super) fn report_diagnostics(path: &Path, source: &str, mut diagnostics: Vec<Diagnostic>) {
    let source_path = path.display().to_string();
    for diagnostic in &mut diagnostics {
        *diagnostic = diagnostic.clone().with_source_path(source_path.clone());
    }
    sort_diagnostics(&mut diagnostics);
    for diagnostic in diagnostics {
        report_diagnostic(path, source, diagnostic);
    }
}

pub(super) fn report_diagnostic(path: &Path, source: &str, diagnostic: Diagnostic) {
    let colored = io::stderr().is_terminal();
    errln!("{}: {}", path.display(), render_colored_diagnostic(source, &diagnostic, colored));
}

/// Reports a finding about a whole file, which has no line and column:
/// `<path>: error[E####]: message`. The path is the diagnostic's own.
pub(super) fn report_unlocated_diagnostic(diagnostic: &Diagnostic) {
    let path = diagnostic.source_path().unwrap_or("<unknown>");
    errln!("{path}: {}", render_unlocated_diagnostic(diagnostic));
}

pub(super) fn report_mapped_diagnostic(
    map: &SharedSourceMap,
    fallback_path: &Path,
    fallback_source: &str,
    diagnostic: Diagnostic,
) {
    let mapped = map.lock().localize(&diagnostic);
    if let Some((path, source, diagnostic)) = mapped {
        report_diagnostic(&path, &source, diagnostic);
    } else {
        report_diagnostic(fallback_path, fallback_source, diagnostic);
    }
}

pub(super) fn report_mapped_diagnostics(
    map: &SharedSourceMap,
    fallback_path: &Path,
    fallback_source: &str,
    diagnostics: Vec<Diagnostic>,
) {
    let localized = diagnostics
        .into_iter()
        .map(|diagnostic| match map.lock().localize(&diagnostic) {
            Some((path, source, diagnostic)) => (path, source, diagnostic),
            None => (
                fallback_path.to_owned(),
                fallback_source.to_owned(),
                diagnostic.with_source_path(fallback_path.display().to_string()),
            ),
        })
        .collect::<Vec<_>>();
    let mut sorted = localized.iter().map(|entry| entry.2.clone()).collect::<Vec<_>>();
    crate::diagnostics::sort_diagnostics(&mut sorted);
    let mut localized = localized;
    for diagnostic in sorted {
        let index = localized
            .iter()
            .position(|entry| entry.2 == diagnostic)
            .expect("localized diagnostic must retain its source");
        let (path, source, diagnostic) = localized.remove(index);
        report_diagnostic(&path, &source, diagnostic);
    }
}

pub(super) fn report_frontend_error(
    source_map: &SharedSourceMap,
    path: &Path,
    source: &str,
    error: &FrontendError,
) {
    match error {
        FrontendError::Lex(errors) => report_mapped_diagnostics(
            source_map,
            path,
            source,
            errors.iter().map(lex_diagnostic).collect(),
        ),
        FrontendError::Parse(error) => {
            report_mapped_diagnostic(source_map, path, source, parse_diagnostic(error));
        }
    }
}
