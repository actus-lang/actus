use crate::lexer::SourceSpan;
use crate::modules::{ModuleError, ModuleResolutionError, PrivateCause};

use crate::diagnostics::DiagnosticRelatedLocation;

use super::{Diagnostic, DiagnosticPhase, lex_diagnostic, parse_diagnostic, semantic_diagnostic};

/// Converts a module, import, or facade failure into the shared diagnostic model.
///
/// Resolution failures use the stable `E1100`–`E1108` module range. Test layout
/// failures use `E1143` and `E1144`. Visibility
/// failures use `E1109`; hierarchical scope failures use `E1119`; conflicting facade openings use `E1137`; configuration policy failures use `E1113`–`E1114`. Nested
/// lexical, parser, and semantic failures retain their subsystem code while
/// gaining the module source path when one is available.
pub fn module_diagnostic(error: &ModuleError) -> Diagnostic {
    match error {
        ModuleError::Resolution(error) => resolution_diagnostic(error),
        ModuleError::UnknownSiblingModule { .. } => {
            module_error("E1104", error.to_string(), SourceSpan::new(0, 0))
        }
        ModuleError::ConfigurationImport { span, path, .. } => {
            module_error("E1113", error.to_string(), *span)
                .with_source_path(path.display().to_string())
        }
        ModuleError::ConfigurationRuntimeDeclaration { span, path, .. } => {
            module_error("E1114", error.to_string(), *span)
                .with_source_path(path.display().to_string())
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
        ModuleError::DuplicateDeclaration(duplicate) => duplicate_diagnostic(error, duplicate),
        ModuleError::TestVerbInProduction { path, span, .. } => {
            module_error("E1144", error.to_string(), *span)
                .with_source_path(path.display().to_string())
        }
        ModuleError::SymbolCollision { .. } => {
            module_error("E1110", error.to_string(), SourceSpan::new(0, 0))
        }
        ModuleError::ImportBindingCollision { first, second, .. } => {
            module_error("E1115", error.to_string(), *second).with_related_location(
                DiagnosticRelatedLocation::new(None, *first, "first import binding is here"),
            )
        }
        ModuleError::DuplicateObjectOwner { .. } => {
            module_error("E1111", error.to_string(), SourceSpan::new(0, 0))
        }
        ModuleError::PrivateDeclarationAccess { .. } => private_access_diagnostic(error),
        ModuleError::ConflictingOpening { span, facade: path, .. } => {
            module_error(visibility_code(error), error.to_string(), *span)
                .with_source_path(path.display().to_string())
        }
        ModuleError::OutOfScopeReference(violation) => {
            module_error(visibility_code(error), error.to_string(), violation.span)
                .with_source_path(violation.path.display().to_string())
        }
        ModuleError::Semantic(error) => semantic_diagnostic(error),
    }
}

fn duplicate_diagnostic(
    error: &ModuleError,
    duplicate: &crate::modules::DuplicateDeclaration,
) -> Diagnostic {
    module_error("E1106", error.to_string(), duplicate.second.span)
        .with_source_path(duplicate.second.path.display().to_string())
        .with_related_location(DiagnosticRelatedLocation::new(
            Some(duplicate.first.path.display().to_string()),
            duplicate.first.span,
            "first declaration is here",
        ))
}

fn private_access_diagnostic(error: &ModuleError) -> Diagnostic {
    let ModuleError::PrivateDeclarationAccess { symbol, span, facade, cause, .. } = error else {
        unreachable!("private access diagnostic received another module error")
    };
    module_error("E1109", error.to_string(), *span)
        .with_source_path(facade.display().to_string())
        .with_suggestion(match &**cause {
            PrivateCause::NotOpen => {
                format!("mark `{symbol}` as `open` in its module to export it")
            }
            PrivateCause::UnitOnly { entry, facade } => format!(
                "`{symbol}` is already `open`, but the facade exports `{entry}` only to its unit; to export it, change `open unit {entry};` to `open {entry};` in `{}`",
                facade.display()
            ),
            PrivateCause::NotOpened { entry, facade } => format!(
                "`{symbol}` is already `open`, but the facade does not export `{entry}`; to export it, add `open {entry};` to `{}`",
                facade.display()
            ),
        })
}

fn visibility_code(error: &ModuleError) -> &'static str {
    match error {
        ModuleError::ConflictingOpening { .. } => "E1137",
        ModuleError::OutOfScopeReference(_) => "E1119",
        _ => "E1109",
    }
}

fn resolution_diagnostic(error: &ModuleResolutionError) -> Diagnostic {
    let (code, path) = match error {
        ModuleResolutionError::InvalidPath(_)
        | ModuleResolutionError::StandardLibraryUnavailable(_) => ("E1100", None),
        ModuleResolutionError::MissingFacade { expected, .. } => {
            ("E1101", Some(expected.display().to_string()))
        }
        ModuleResolutionError::IncompatibleRuntime { expected, .. } => {
            ("E1112", Some(expected.display().to_string()))
        }
        ModuleResolutionError::AmbiguousModule { directory, .. } => {
            ("E1102", Some(directory.display().to_string()))
        }
        ModuleResolutionError::BypassesFacade { facade, .. } => {
            ("E1108", Some(facade.display().to_string()))
        }
        ModuleResolutionError::Io { path, .. } => ("E1103", Some(path.display().to_string())),
        ModuleResolutionError::TestLayout { path, .. } => {
            ("E1143", Some(path.display().to_string()))
        }
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
