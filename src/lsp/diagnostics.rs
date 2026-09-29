use std::path::{Path, PathBuf};

use crate::configuration::CompilerConfiguration;
use crate::diagnostics::{
    Diagnostic, DiagnosticSeverity, lex_diagnostic, module_diagnostic, parse_diagnostic,
    render_diagnostic, semantic_diagnostic, sort_diagnostics,
};
use crate::lexer::scan;
use crate::modules::{ModuleError, ModuleResolver};
use crate::parser::parse;
use crate::semantic::analyze;

use super::position::{LineIndex, LspRange};
use super::protocol::LspDiagnostic;

/// Analyzes one document and converts the shared diagnostic model to LSP values.
///
/// `uri` identifies the document, `source` is the current document text, and
/// `overlays` supplies unsaved sibling modules for package-aware analysis.
/// The returned diagnostics preserve code, severity, and source range while
/// rendering the human-readable message through the shared text renderer.
pub fn analyze_document(
    uri: &str,
    source: &str,
    overlays: &std::collections::HashMap<PathBuf, String>,
) -> Vec<LspDiagnostic> {
    let diagnostics = collect_diagnostics(uri, source, overlays);
    let index = LineIndex::new(source);
    diagnostics
        .into_iter()
        .map(|diagnostic| to_lsp_diagnostic(source, &index, &diagnostic))
        .collect()
}

fn collect_diagnostics(
    uri: &str,
    source: &str,
    overlays: &std::collections::HashMap<PathBuf, String>,
) -> Vec<Diagnostic> {
    let (tokens, lex_errors) = scan(source);
    if !lex_errors.is_empty() {
        let mut diagnostics: Vec<Diagnostic> = lex_errors.iter().map(lex_diagnostic).collect();
        sort_diagnostics(&mut diagnostics);
        return diagnostics;
    }
    let program = match parse(tokens) {
        Ok(program) => program,
        Err(error) => {
            return vec![parse_diagnostic(&error)];
        }
    };
    if let Some(result) = analyze_package_module(uri, &program, overlays) {
        let mut diagnostics = package_diagnostics(result);
        sort_diagnostics(&mut diagnostics);
        return diagnostics;
    }
    if let Some(result) = analyze_test_fixture(uri, &program) {
        let mut diagnostics = package_diagnostics(result);
        sort_diagnostics(&mut diagnostics);
        return diagnostics;
    }
    if let Err(error) = analyze(&program) {
        return vec![semantic_diagnostic(&error)];
    }
    Vec::new()
}

fn analyze_test_fixture(
    uri: &str,
    program: &crate::ast::Program,
) -> Option<Result<(), ModuleError>> {
    let path = file_uri_to_path(uri)?;
    let configuration = CompilerConfiguration::from_input_path_read_only(&path).ok()?;
    let tests_root = configuration.project_root().join("tests");
    if !path.starts_with(tests_root) {
        return None;
    }
    let resolver = ModuleResolver::with_dependencies(
        configuration.source_root(),
        configuration.dependency_roots(),
    );
    Some(crate::modules::analyze_with_imports(program, &resolver).map(|_| ()))
}

fn analyze_package_module(
    uri: &str,
    program: &crate::ast::Program,
    overlays: &std::collections::HashMap<PathBuf, String>,
) -> Option<Result<(), ModuleError>> {
    let path = file_uri_to_path(uri)?;
    let configuration = CompilerConfiguration::from_input_path_read_only(&path).ok()?;
    let resolver = ModuleResolver::with_dependencies(
        configuration.source_root(),
        configuration.dependency_roots(),
    );
    let result = match module_path_for_file(configuration.source_root(), &path) {
        Some(module_path) => {
            crate::modules::analyze_module_with_overlays(&resolver, &module_path, overlays)
        }
        None if path.parent() == Some(configuration.source_root()) => {
            crate::modules::analyze_with_imports(program, &resolver)
        }
        None if is_project_entry_with_imports(&path, program, configuration.project_root()) => {
            crate::modules::analyze_with_imports(program, &resolver)
        }
        None => return None,
    };
    Some(result.map(|_| ()))
}

fn is_project_entry_with_imports(
    path: &Path,
    program: &crate::ast::Program,
    project_root: &Path,
) -> bool {
    path.starts_with(project_root)
        && !path.starts_with(project_root.join("tests"))
        && program
            .declarations
            .iter()
            .any(|declaration| matches!(declaration, crate::ast::TopLevelDecl::Import(_)))
}

fn package_diagnostics(result: Result<(), ModuleError>) -> Vec<Diagnostic> {
    match result {
        Ok(()) => Vec::new(),
        Err(error) => vec![module_diagnostic(&error)],
    }
}

fn clamp_span(span: crate::lexer::SourceSpan, source_length: usize) -> crate::lexer::SourceSpan {
    let start = span.start.min(source_length);
    let end = span.end.min(source_length).max(start);
    crate::lexer::SourceSpan::new(start, end)
}

fn module_path_for_file(source_root: &Path, file: &Path) -> Option<String> {
    file.strip_prefix(source_root).ok()?;
    let mut directory = file.parent()?;
    while directory.starts_with(source_root) && directory != source_root {
        let name = directory.file_name()?.to_str()?;
        if directory.join(format!("{name}.act")).is_file() {
            let module = directory.strip_prefix(source_root).ok()?;
            return Some(path_components(module).join("::"));
        }
        directory = directory.parent()?;
    }
    None
}

fn path_components(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|component| component.as_os_str().to_str().map(str::to_owned))
        .collect()
}

fn file_uri_to_path(uri: &str) -> Option<PathBuf> {
    let path = uri.strip_prefix("file://")?;
    #[cfg(windows)]
    {
        Some(PathBuf::from(path.trim_start_matches('/').replace('/', "\\")))
    }
    #[cfg(not(windows))]
    Some(PathBuf::from(path))
}

fn to_lsp_diagnostic(source: &str, index: &LineIndex, diagnostic: &Diagnostic) -> LspDiagnostic {
    let span = clamp_span(diagnostic.span(), source.len());
    let start = index.position(source, span.start);
    let end = index.position(source, span.end.max(span.start));
    LspDiagnostic {
        range: LspRange { start, end },
        severity: lsp_severity(diagnostic.severity()),
        code: Some(diagnostic.code().to_owned()),
        source: Some("actus".to_owned()),
        message: render_diagnostic(source, diagnostic),
    }
}

fn lsp_severity(severity: DiagnosticSeverity) -> u8 {
    match severity {
        DiagnosticSeverity::Error => 1,
        DiagnosticSeverity::Warning => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::lsp_severity;
    use crate::diagnostics::DiagnosticSeverity;

    #[test]
    fn maps_diagnostic_severity_to_lsp_values() {
        assert_eq!(lsp_severity(DiagnosticSeverity::Error), 1);
        assert_eq!(lsp_severity(DiagnosticSeverity::Warning), 2);
    }
}
