use serde_json::{Value, json};

use crate::lexer::SourceSpan;

use super::super::{Diagnostic, DiagnosticPhase, DiagnosticSeverity, sort_diagnostics};

/// Serializes diagnostics into the stable machine-readable JSON contract.
///
/// The input is not mutated. A sorted copy is serialized so choosing JSON as
/// a renderer cannot change ordering observed by another renderer.
pub fn render_json_diagnostics(diagnostics: &[Diagnostic]) -> Result<String, serde_json::Error> {
    let mut sorted = diagnostics.to_vec();
    sort_diagnostics(&mut sorted);
    let values = sorted.iter().map(json_diagnostic).collect::<Vec<_>>();
    serde_json::to_string(&values)
}

fn json_diagnostic(diagnostic: &Diagnostic) -> Value {
    json!({
        "code": diagnostic.code(),
        "explanation": diagnostic.explanation(),
        "message": diagnostic.message(),
        "phase": phase_name(diagnostic.phase()),
        "severity": severity_name(diagnostic.severity()),
        "sourcePath": diagnostic.source_path(),
        "span": json_span(diagnostic.span()),
        "suggestion": diagnostic.suggestion(),
        "relatedInformation": diagnostic
            .related_locations()
            .iter()
            .map(json_related_location)
            .collect::<Vec<_>>(),
    })
}

fn json_related_location(location: &super::super::DiagnosticRelatedLocation) -> Value {
    json!({
        "message": location.message(),
        "sourcePath": location.source_path(),
        "span": json_span(location.span()),
    })
}

fn json_span(span: SourceSpan) -> Value {
    json!({ "start": span.start, "end": span.end })
}

fn phase_name(phase: DiagnosticPhase) -> &'static str {
    match phase {
        DiagnosticPhase::Configuration => "configuration",
        DiagnosticPhase::Conformance => "conformance",
        DiagnosticPhase::Lexical => "lexical",
        DiagnosticPhase::Parser => "parser",
        DiagnosticPhase::Module => "module",
        DiagnosticPhase::Target => "target",
        DiagnosticPhase::Semantic => "semantic",
        DiagnosticPhase::Codegen => "codegen",
        DiagnosticPhase::Unclassified => "unclassified",
    }
}

fn severity_name(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Error => "error",
        DiagnosticSeverity::Warning => "warning",
    }
}
