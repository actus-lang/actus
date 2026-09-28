use std::path::Path;

use crate::diagnostics::{Diagnostic, render_diagnostic, sort_diagnostics};

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
    eprintln!("{}: {}", path.display(), render_diagnostic(source, &diagnostic));
}
