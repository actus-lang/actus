use crate::ast::{LiteralPattern, Pattern};
use crate::lexer::SourceSpan;

use super::errors::{SemanticError, SemanticErrorKind};

pub(super) fn pattern_span(pattern: &Pattern) -> SourceSpan {
    match pattern {
        Pattern::Variant { span, .. }
        | Pattern::Literal { span, .. }
        | Pattern::Wildcard { span } => *span,
    }
}

pub(super) fn pattern_name(pattern: &Pattern) -> String {
    match pattern {
        Pattern::Variant { enum_name, variant, .. } => format!("{enum_name}.{variant}"),
        Pattern::Literal { value, .. } => match value {
            LiteralPattern::Integer(value) => value.clone(),
            LiteralPattern::Bool(value) => value.to_string(),
        },
        Pattern::Wildcard { .. } => "_".to_owned(),
    }
}

pub(super) fn variant_key(pattern: &Pattern, enum_name: &str) -> Option<String> {
    match pattern {
        Pattern::Variant { enum_name: found, variant, .. } if found == enum_name => {
            Some(variant.clone())
        }
        _ => None,
    }
}

pub(super) fn is_wildcard(pattern: &Pattern) -> bool {
    matches!(pattern, Pattern::Wildcard { .. })
}

pub(super) fn non_exhaustive(
    subject: &str,
    missing: Vec<String>,
    span: SourceSpan,
) -> SemanticError {
    SemanticError {
        kind: SemanticErrorKind::NonExhaustiveMatch { subject: subject.to_owned(), missing },
        span,
    }
}

pub(super) fn unreachable_pattern(pattern: String, span: SourceSpan) -> SemanticError {
    SemanticError { kind: SemanticErrorKind::UnreachablePattern { pattern }, span }
}

pub(super) fn duplicate_pattern(pattern: String, span: SourceSpan) -> SemanticError {
    SemanticError { kind: SemanticErrorKind::DuplicatePattern { pattern }, span }
}

pub(super) fn pattern_type_mismatch(
    expected: &str,
    found: &str,
    span: SourceSpan,
) -> SemanticError {
    SemanticError {
        kind: SemanticErrorKind::PatternTypeMismatch {
            expected: expected.to_owned(),
            found: found.to_owned(),
        },
        span,
    }
}
