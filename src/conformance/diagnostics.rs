use std::path::Path;

use crate::diagnostics::{
    Diagnostic, DiagnosticPhase, STRICT_SOURCE_FILE_DECOMPOSITION, STRICT_SOURCE_FILE_HARD_LIMIT,
    STRICT_SOURCE_FILE_SPLIT_REQUIRED, STRICT_SOURCE_FUNCTION_DECOMPOSITION,
    STRICT_SOURCE_FUNCTION_HARD_LIMIT, STRICT_SOURCE_FUNCTION_SPLIT_REQUIRED,
    STRICT_SOURCE_LIMIT_SUPPRESSION,
};
use crate::lexer::SourceSpan;

use super::policy::SourceLimitPolicy;
use super::scan::{FunctionRange, code_line_count_between, line_span};

pub(super) fn file_diagnostics(
    path: &Path,
    source: &str,
    line_count: usize,
    policy: SourceLimitPolicy,
) -> Vec<Diagnostic> {
    let Some((code, message, explanation)) = file_violation(line_count, policy) else {
        return Vec::new();
    };
    vec![diagnostic(path, SourceSpan::new(0, source.len()), code, message, explanation)]
}

pub(super) fn function_diagnostics(
    path: &Path,
    source: &str,
    function: &FunctionRange,
    policy: SourceLimitPolicy,
    hash_comments: bool,
) -> Vec<Diagnostic> {
    let line_count =
        code_line_count_between(source, function.start_line, function.end_line, hash_comments);
    let Some((code, message, explanation)) = function_violation(line_count, &function.name, policy)
    else {
        return Vec::new();
    };
    let span = line_span(source, function.start_line, function.end_line);
    vec![diagnostic(path, span, code, message, explanation)]
}

pub(super) fn suppression_diagnostics(
    path: &Path,
    source: &str,
    hash_comments: bool,
) -> Vec<Diagnostic> {
    const MARKERS: [&str; 6] = [
        "actus: allow(source-limit)",
        "actus: ignore(source-limit)",
        "# actus: allow(source-limit)",
        "# actus: ignore(source-limit)",
        "#[allow(source_limit)]",
        "#![allow(source_limit)]",
    ];
    let mut diagnostics = Vec::new();
    for (line_number, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        let hash_comment = hash_comments && trimmed.starts_with('#') && !trimmed.starts_with("#[");
        let comment_or_attribute =
            trimmed.starts_with("//") || trimmed.starts_with("#[") || hash_comment;
        if comment_or_attribute && MARKERS.iter().any(|marker| line.contains(marker)) {
            let span = line_span(source, line_number + 1, line_number + 1);
            diagnostics.push(
                Diagnostic::error(
                    STRICT_SOURCE_LIMIT_SUPPRESSION,
                    span,
                    "source-local suppression of architectural limits is forbidden",
                )
                .with_source_path(path.display().to_string())
                .with_phase(DiagnosticPhase::Conformance)
                .with_explanation(
                    "Limit exceptions must be declared in the reviewed conformance manifest.",
                )
                .with_suggestion("Remove the suppression and split the source by responsibility."),
            );
        }
    }
    diagnostics
}

fn file_violation(
    line_count: usize,
    policy: SourceLimitPolicy,
) -> Option<(&'static str, String, &'static str)> {
    if line_count > policy.hard_file_lines {
        return Some((
            STRICT_SOURCE_FILE_HARD_LIMIT,
            format!(
                "source file has {line_count} code lines; hard limit is {}",
                policy.hard_file_lines
            ),
            "A source file above the hard limit cannot be accepted by strict conformance.",
        ));
    }
    if line_count >= policy.file_split_lines {
        return Some((
            STRICT_SOURCE_FILE_SPLIT_REQUIRED,
            format!(
                "source file has {line_count} code lines; split is required at {}",
                policy.file_split_lines
            ),
            "Split the file by responsibility before adding more implementation.",
        ));
    }
    (line_count > policy.preferred_file_lines).then_some((
        STRICT_SOURCE_FILE_DECOMPOSITION,
        format!(
            "source file has {line_count} code lines; preferred limit is {}",
            policy.preferred_file_lines
        ),
        "Record decomposition evidence or split the file by responsibility.",
    ))
}

fn function_violation(
    line_count: usize,
    function_name: &str,
    policy: SourceLimitPolicy,
) -> Option<(&'static str, String, &'static str)> {
    hard_function_violation(line_count, function_name, policy)
        .or_else(|| split_function_violation(line_count, function_name, policy))
        .or_else(|| preferred_function_violation(line_count, function_name, policy))
}

fn hard_function_violation(
    line_count: usize,
    function_name: &str,
    policy: SourceLimitPolicy,
) -> Option<(&'static str, String, &'static str)> {
    (line_count > policy.hard_function_lines).then_some((
        STRICT_SOURCE_FUNCTION_HARD_LIMIT,
        format!(
            "function `{function_name}` has {line_count} code lines; hard limit is {}",
            policy.hard_function_lines
        ),
        "Split the function into responsibility-specific operations.",
    ))
}

fn split_function_violation(
    line_count: usize,
    function_name: &str,
    policy: SourceLimitPolicy,
) -> Option<(&'static str, String, &'static str)> {
    (line_count >= policy.function_split_lines).then_some((
        STRICT_SOURCE_FUNCTION_SPLIT_REQUIRED,
        format!(
            "function `{function_name}` has {line_count} code lines; split is required at {}",
            policy.function_split_lines
        ),
        "Extract independent validation or transformation stages into named functions.",
    ))
}

fn preferred_function_violation(
    line_count: usize,
    function_name: &str,
    policy: SourceLimitPolicy,
) -> Option<(&'static str, String, &'static str)> {
    (line_count > policy.preferred_function_lines).then_some((
        STRICT_SOURCE_FUNCTION_DECOMPOSITION,
        format!(
            "function `{function_name}` has {line_count} code lines; preferred limit is {}",
            policy.preferred_function_lines
        ),
        "Record decomposition evidence or split the function by responsibility.",
    ))
}

fn diagnostic(
    path: &Path,
    span: SourceSpan,
    code: &str,
    message: String,
    explanation: &str,
) -> Diagnostic {
    let diagnostic = match code {
        STRICT_SOURCE_FILE_SPLIT_REQUIRED
        | STRICT_SOURCE_FILE_HARD_LIMIT
        | STRICT_SOURCE_FUNCTION_SPLIT_REQUIRED
        | STRICT_SOURCE_FUNCTION_HARD_LIMIT => Diagnostic::error(code, span, message),
        _ => Diagnostic::warning(code, span, message),
    };
    diagnostic
        .with_source_path(path.display().to_string())
        .with_phase(DiagnosticPhase::Conformance)
        .with_explanation(explanation)
        .with_suggestion("Reduce the source size or document an approved structural exception.")
}
