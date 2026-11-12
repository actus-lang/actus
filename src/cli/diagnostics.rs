use std::io::{self, IsTerminal};
use std::path::Path;

use crate::SharedSourceMap;
use crate::diagnostics::{Diagnostic, render_colored_diagnostic, sort_diagnostics};

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
    eprintln!("{}: {}", path.display(), render_colored_diagnostic(source, &diagnostic, colored));
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
