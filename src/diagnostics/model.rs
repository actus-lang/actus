use crate::lexer::SourceSpan;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
/// The compiler phase that produced a validation diagnostic.
pub enum DiagnosticPhase {
    /// A configuration or workspace-policy validation failure.
    Configuration,
    /// A lexical scanning failure.
    Lexical,
    /// A syntax parsing failure.
    Parser,
    /// A module discovery or import-resolution failure.
    Module,
    /// A semantic validation failure.
    Semantic,
    /// A diagnostic whose producer has not assigned a phase yet.
    Unclassified,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
/// The severity used by a validation diagnostic.
pub enum DiagnosticSeverity {
    /// A violation that makes the current validation result unsuccessful.
    Error,
    /// A condition that may be promoted to an error by strict policy.
    Warning,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// A renderer-independent compiler diagnostic.
///
/// The model preserves the stable code, severity, source span, optional source
/// path, human-readable message, explanation, and suggested correction. CLI,
/// colored terminal, JSON, and LSP renderers may format this value differently
/// without changing validation semantics.
pub struct Diagnostic {
    code: String,
    severity: DiagnosticSeverity,
    phase: DiagnosticPhase,
    span: SourceSpan,
    source_path: Option<String>,
    message: String,
    explanation: Option<String>,
    suggestion: Option<String>,
}

impl Diagnostic {
    /// Creates an error diagnostic with no optional explanation or correction.
    pub fn error(code: impl Into<String>, span: SourceSpan, message: impl Into<String>) -> Self {
        Self::new(code, DiagnosticSeverity::Error, span, message)
    }

    /// Creates a warning diagnostic that strict policy may promote to failure.
    pub fn warning(code: impl Into<String>, span: SourceSpan, message: impl Into<String>) -> Self {
        Self::new(code, DiagnosticSeverity::Warning, span, message)
    }

    /// Attaches the canonical source path associated with this diagnostic.
    pub fn with_source_path(mut self, path: impl Into<String>) -> Self {
        self.source_path = Some(path.into());
        self
    }

    /// Attaches the compiler phase that produced this diagnostic.
    pub fn with_phase(mut self, phase: DiagnosticPhase) -> Self {
        self.phase = phase;
        self
    }

    /// Attaches a stable explanation of the violated language or architecture
    /// rule.
    pub fn with_explanation(mut self, explanation: impl Into<String>) -> Self {
        self.explanation = Some(explanation.into());
        self
    }

    /// Attaches an actionable correction without changing the diagnostic code.
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Returns the stable diagnostic code.
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Returns the validation severity.
    pub const fn severity(&self) -> DiagnosticSeverity {
        self.severity
    }

    /// Returns the compiler phase that produced this diagnostic.
    pub const fn phase(&self) -> DiagnosticPhase {
        self.phase
    }

    /// Returns the half-open byte span in the original source.
    pub const fn span(&self) -> SourceSpan {
        self.span
    }

    /// Returns the optional canonical source path.
    pub fn source_path(&self) -> Option<&str> {
        self.source_path.as_deref()
    }

    /// Returns the primary human-readable message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the optional explanation of the violated rule.
    pub fn explanation(&self) -> Option<&str> {
        self.explanation.as_deref()
    }

    /// Returns the optional actionable correction.
    pub fn suggestion(&self) -> Option<&str> {
        self.suggestion.as_deref()
    }

    fn new(
        code: impl Into<String>,
        severity: DiagnosticSeverity,
        span: SourceSpan,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity,
            phase: DiagnosticPhase::Unclassified,
            span,
            source_path: None,
            message: message.into(),
            explanation: None,
            suggestion: None,
        }
    }
}

/// Sorts diagnostics by stable source identity, location, phase, and code.
///
/// The message is used only as a final tie-breaker so repeated diagnostics
/// with identical primary metadata still have a deterministic order.
pub fn sort_diagnostics(diagnostics: &mut [Diagnostic]) {
    diagnostics.sort_by(|left, right| {
        left.source_path()
            .cmp(&right.source_path())
            .then(left.span().start.cmp(&right.span().start))
            .then(left.span().end.cmp(&right.span().end))
            .then(left.phase().cmp(&right.phase()))
            .then(left.code().cmp(right.code()))
            .then(left.message().cmp(right.message()))
    });
}
