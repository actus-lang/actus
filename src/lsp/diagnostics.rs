use std::path::{Path, PathBuf};

use crate::configuration::CompilerConfiguration;
use crate::diagnostics::{render_lex_error, render_parse_error, render_semantic_error};
use crate::lexer::scan;
use crate::modules::{ModuleError, ModuleResolver};
use crate::parser::parse;
use crate::semantic::analyze;

use super::position::{LineIndex, LspRange};
use super::protocol::LspDiagnostic;

pub fn analyze_document(
    uri: &str,
    source: &str,
    overlays: &std::collections::HashMap<PathBuf, String>,
) -> Vec<LspDiagnostic> {
    let (tokens, lex_errors) = scan(source);
    let index = LineIndex::new(source);
    if !lex_errors.is_empty() {
        return lex_errors
            .iter()
            .map(|error| diagnostic(source, &index, error.span, render_lex_error(source, error)))
            .collect();
    }
    let program = match parse(tokens) {
        Ok(program) => program,
        Err(error) => {
            return vec![diagnostic(
                source,
                &index,
                error.span,
                render_parse_error(source, &error),
            )];
        }
    };
    if let Some(result) = analyze_package_module(uri, overlays) {
        return package_diagnostics(source, &index, result);
    }
    if let Err(error) = analyze(&program) {
        return vec![diagnostic(source, &index, error.span, render_semantic_error(source, &error))];
    }
    Vec::new()
}

fn analyze_package_module(
    uri: &str,
    overlays: &std::collections::HashMap<PathBuf, String>,
) -> Option<Result<(), ModuleError>> {
    let path = file_uri_to_path(uri)?;
    let configuration = CompilerConfiguration::from_input_path_read_only(&path).ok()?;
    let module_path = module_path_for_file(configuration.source_root(), &path)?;
    let resolver = ModuleResolver::with_dependencies(
        configuration.source_root(),
        configuration.dependency_roots(),
    );
    Some(
        crate::modules::analyze_module_with_overlays(&resolver, &module_path, overlays).map(|_| ()),
    )
}

fn package_diagnostics(
    source: &str,
    index: &LineIndex,
    result: Result<(), ModuleError>,
) -> Vec<LspDiagnostic> {
    match result {
        Ok(()) => Vec::new(),
        Err(ModuleError::Semantic(error)) => {
            let span = clamp_span(error.span, source.len());
            vec![diagnostic(source, index, span, render_semantic_error(source, &error))]
        }
        Err(error) => vec![diagnostic(
            source,
            index,
            crate::lexer::SourceSpan::new(0, 0),
            format!("error[E1000]: {error}"),
        )],
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

fn diagnostic(
    source: &str,
    index: &LineIndex,
    span: crate::lexer::SourceSpan,
    message: String,
) -> LspDiagnostic {
    let start = index.position(source, span.start);
    let end = index.position(source, span.end.max(span.start));
    LspDiagnostic {
        range: LspRange { start, end },
        severity: 1,
        code: diagnostic_code(&message),
        source: Some("actus".to_owned()),
        message,
    }
}

fn diagnostic_code(message: &str) -> Option<String> {
    let start = message.find('[')? + 1;
    let end = message[start..].find(']')? + start;
    Some(message[start..end].to_owned())
}

#[cfg(test)]
mod tests {
    use super::diagnostic_code;

    #[test]
    fn preserves_ownership_diagnostic_codes_for_lsp() {
        for code in ["E1064", "E1065", "E1066", "E1009", "E1011"] {
            let message = format!("error[{code}] at 1:1: ownership violation");
            assert_eq!(diagnostic_code(&message).as_deref(), Some(code));
        }
    }
}
