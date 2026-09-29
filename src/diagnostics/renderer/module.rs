use crate::lexer::SourceSpan;
use crate::modules::{ModuleError, ModuleResolutionError};

use super::{Diagnostic, DiagnosticPhase, lex_diagnostic, parse_diagnostic, semantic_diagnostic};

/// Converts a module, import, or facade failure into the shared diagnostic model.
///
/// Resolution failures use the stable `E1100`–`E1108` module range. Visibility
/// failures use `E1109`. Nested
/// lexical, parser, and semantic failures retain their subsystem code while
/// gaining the module source path when one is available.
pub fn module_diagnostic(error: &ModuleError) -> Diagnostic {
    match error {
        ModuleError::Resolution(error) => resolution_diagnostic(error),
        ModuleError::UnknownSiblingModule { .. } => {
            module_error("E1104", error.to_string(), SourceSpan::new(0, 0))
        }
        ModuleError::Read { path, .. } => {
            module_error("E1105", error.to_string(), SourceSpan::new(0, 0))
                .with_source_path(path.display().to_string())
        }
        ModuleError::Lex { path, errors } => errors
            .first()
            .map(lex_diagnostic)
            .unwrap_or_else(|| module_error("E1107", error.to_string(), SourceSpan::new(0, 0)))
            .with_source_path(path.display().to_string()),
        ModuleError::Parse { path, error: parse_error } => {
            parse_diagnostic(parse_error).with_source_path(path.display().to_string())
        }
        ModuleError::DuplicateDeclaration(diagnostic) => {
            module_error("E1106", error.to_string(), diagnostic.second.span)
                .with_source_path(diagnostic.second.path.display().to_string())
        }
        ModuleError::SymbolCollision { .. } => {
            module_error("E1110", error.to_string(), SourceSpan::new(0, 0))
        }
        ModuleError::PrivateDeclarationAccess { span, facade, .. } => {
            module_error("E1109", error.to_string(), *span)
                .with_source_path(facade.display().to_string())
        }
        ModuleError::Semantic(error) => semantic_diagnostic(error),
    }
}

fn resolution_diagnostic(error: &ModuleResolutionError) -> Diagnostic {
    let (code, path) = match error {
        ModuleResolutionError::InvalidPath(_) => ("E1100", None),
        ModuleResolutionError::MissingFacade { expected, .. } => {
            ("E1101", Some(expected.display().to_string()))
        }
        ModuleResolutionError::AmbiguousModule { directory, .. } => {
            ("E1102", Some(directory.display().to_string()))
        }
        ModuleResolutionError::BypassesFacade { facade, .. } => {
            ("E1108", Some(facade.display().to_string()))
        }
        ModuleResolutionError::Io { path, .. } => ("E1103", Some(path.display().to_string())),
    };
    let diagnostic = module_error(code, error.to_string(), SourceSpan::new(0, 0));
    match path {
        Some(path) => diagnostic.with_source_path(path),
        None => diagnostic,
    }
}

fn module_error(code: &'static str, message: String, span: SourceSpan) -> Diagnostic {
    Diagnostic::error(code, span, message).with_phase(DiagnosticPhase::Module)
}
