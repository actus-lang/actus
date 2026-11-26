use crate::diagnostics::DiagnosticCode;
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
        ModuleError::Lex { path, errors } => errors
            .first()
            .map(lex_diagnostic)
            .unwrap_or_else(|| {
                module_error(
                    DiagnosticCode::MODULE_LEX_FAILURE,
                    error.to_string(),
                    SourceSpan::new(0, 0),
                )
            })
            .with_source_path(path.display().to_string()),
        ModuleError::Parse { path, error: parse_error } => {
            parse_diagnostic(parse_error).with_source_path(path.display().to_string())
        }
        ModuleError::DuplicateDeclaration(duplicate) => duplicate_diagnostic(error, duplicate),
        ModuleError::PrivateDeclarationAccess { .. } => private_access_diagnostic(error),
        ModuleError::ConflictingOpening { span, facade: path, .. } => {
            module_error(visibility_code(error), error.to_string(), *span)
                .with_source_path(path.display().to_string())
        }
        ModuleError::OutOfScopeReference(violation) => {
            module_error(visibility_code(error), error.to_string(), violation.span)
                .with_source_path(violation.path.display().to_string())
        }
        ModuleError::ConfigurationImport { .. }
        | ModuleError::ConfigurationRuntimeDeclaration { .. }
        | ModuleError::Read { .. }
        | ModuleError::TestVerbInProduction { .. } => located_failure_diagnostic(error),
        ModuleError::UnknownSiblingModule { .. }
        | ModuleError::SymbolCollision { .. }
        | ModuleError::ImportBindingCollision { .. }
        | ModuleError::DuplicateObjectOwner { .. } => unlocated_failure_diagnostic(error),
        ModuleError::Semantic(error) => semantic_diagnostic(error),
    }
}

/// Failures that name one module source file.
fn located_failure_diagnostic(error: &ModuleError) -> Diagnostic {
    match error {
        ModuleError::ConfigurationImport { span, path, .. } => {
            module_error(DiagnosticCode::MODULE_CONFIGURATION_IMPORT, error.to_string(), *span)
                .with_source_path(path.display().to_string())
        }
        ModuleError::ConfigurationRuntimeDeclaration { span, path, .. } => module_error(
            DiagnosticCode::MODULE_CONFIGURATION_RUNTIME_DECLARATION,
            error.to_string(),
            *span,
        )
        .with_source_path(path.display().to_string()),
        ModuleError::Read { path, .. } => module_error(
            DiagnosticCode::MODULE_READ_FAILURE,
            error.to_string(),
            SourceSpan::new(0, 0),
        )
        .with_source_path(path.display().to_string()),
        ModuleError::TestVerbInProduction { path, span, .. } => {
            module_error(DiagnosticCode::MODULE_TEST_VERB_IN_PRODUCTION, error.to_string(), *span)
                .with_source_path(path.display().to_string())
        }
        _ => unreachable!("located failure diagnostic received another module error"),
    }
}

/// Failures between declarations, reported without a source file of their own.
fn unlocated_failure_diagnostic(error: &ModuleError) -> Diagnostic {
    match error {
        ModuleError::UnknownSiblingModule { .. } => module_error(
            DiagnosticCode::MODULE_UNKNOWN_SIBLING,
            error.to_string(),
            SourceSpan::new(0, 0),
        ),
        ModuleError::SymbolCollision { .. } => module_error(
            DiagnosticCode::MODULE_SYMBOL_COLLISION,
            error.to_string(),
            SourceSpan::new(0, 0),
        ),
        ModuleError::ImportBindingCollision { first, second, .. } => module_error(
            DiagnosticCode::MODULE_IMPORT_BINDING_COLLISION,
            error.to_string(),
            *second,
        )
        .with_related_location(DiagnosticRelatedLocation::new(
            None,
            *first,
            "first import binding is here",
        )),
        ModuleError::DuplicateObjectOwner { .. } => module_error(
            DiagnosticCode::MODULE_DUPLICATE_OBJECT_OWNER,
            error.to_string(),
            SourceSpan::new(0, 0),
        ),
        _ => unreachable!("unlocated failure diagnostic received another module error"),
    }
}

fn duplicate_diagnostic(
    error: &ModuleError,
    duplicate: &crate::modules::DuplicateDeclaration,
) -> Diagnostic {
    module_error(
        DiagnosticCode::MODULE_DUPLICATE_DECLARATION,
        error.to_string(),
        duplicate.second.span,
    )
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
    module_error(DiagnosticCode::MODULE_PRIVATE_DECLARATION_ACCESS, error.to_string(), *span)
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

fn visibility_code(error: &ModuleError) -> DiagnosticCode {
    match error {
        ModuleError::ConflictingOpening { .. } => DiagnosticCode::MODULE_CONFLICTING_OPENING,
        ModuleError::OutOfScopeReference(_) => DiagnosticCode::MODULE_OUT_OF_SCOPE_REFERENCE,
        _ => DiagnosticCode::MODULE_PRIVATE_DECLARATION_ACCESS,
    }
}

fn resolution_diagnostic(error: &ModuleResolutionError) -> Diagnostic {
    let (code, path) = match error {
        ModuleResolutionError::InvalidPath(_)
        | ModuleResolutionError::StandardLibraryUnavailable(_) => {
            (DiagnosticCode::MODULE_PATH_UNRESOLVED, None)
        }
        ModuleResolutionError::MissingFacade { expected, .. } => {
            (DiagnosticCode::MODULE_MISSING_FACADE, Some(expected.display().to_string()))
        }
        ModuleResolutionError::IncompatibleRuntime { expected, .. } => {
            (DiagnosticCode::MODULE_INCOMPATIBLE_RUNTIME, Some(expected.display().to_string()))
        }
        ModuleResolutionError::AmbiguousModule { directory, .. } => {
            (DiagnosticCode::MODULE_AMBIGUOUS_MODULE, Some(directory.display().to_string()))
        }
        ModuleResolutionError::BypassesFacade { facade, .. } => {
            (DiagnosticCode::MODULE_BYPASSES_FACADE, Some(facade.display().to_string()))
        }
        ModuleResolutionError::Io { path, .. } => {
            (DiagnosticCode::MODULE_IO_FAILURE, Some(path.display().to_string()))
        }
        ModuleResolutionError::TestLayout { path, .. } => {
            (DiagnosticCode::MODULE_TEST_LAYOUT, Some(path.display().to_string()))
        }
    };
    let diagnostic = module_error(code, error.to_string(), SourceSpan::new(0, 0));
    match path {
        Some(path) => diagnostic.with_source_path(path),
        None => diagnostic,
    }
}

fn module_error(code: DiagnosticCode, message: String, span: SourceSpan) -> Diagnostic {
    Diagnostic::error(code.as_str(), span, message).with_phase(DiagnosticPhase::Module)
}
