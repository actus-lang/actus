use std::path::Path;

use crate::diagnostics::{
    Diagnostic, DiagnosticPhase, STRICT_SOURCE_FILE_DECOMPOSITION, STRICT_SOURCE_FILE_HARD_LIMIT,
    STRICT_SOURCE_FILE_SPLIT_REQUIRED, STRICT_SOURCE_FUNCTION_DECOMPOSITION,
    STRICT_SOURCE_FUNCTION_HARD_LIMIT, STRICT_SOURCE_FUNCTION_SPLIT_REQUIRED,
};
use crate::lexer::SourceSpan;

use super::policy::SourceLimitPolicy;
use super::scan::{FunctionRange, line_span};

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
) -> Vec<Diagnostic> {
    let line_count = function.end_line - function.start_line + 1;
    let Some((code, message, explanation)) = function_violation(line_count, &function.name, policy)
    else {
        return Vec::new();
    };
    let span = line_span(source, function.start_line, function.end_line);
    vec![diagnostic(path, span, code, message, explanation)]
}

fn file_violation(
    line_count: usize,
    policy: SourceLimitPolicy,
) -> Option<(&'static str, String, &'static str)> {
    if line_count > policy.hard_file_lines {
        return Some((
            STRICT_SOURCE_FILE_HARD_LIMIT,
            format!("source file has {line_count} lines; hard limit is {}", policy.hard_file_lines),
            "A source file above the hard limit cannot be accepted by strict conformance.",
        ));
    }
    if line_count >= policy.file_split_lines {
        return Some((
            STRICT_SOURCE_FILE_SPLIT_REQUIRED,
            format!(
                "source file has {line_count} lines; split is required at {}",
                policy.file_split_lines
            ),
            "Split the file by responsibility before adding more implementation.",
        ));
    }
    (line_count > policy.preferred_file_lines).then_some((
        STRICT_SOURCE_FILE_DECOMPOSITION,
        format!(
            "source file has {line_count} lines; preferred limit is {}",
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
    if line_count > policy.hard_function_lines {
        return Some((
            STRICT_SOURCE_FUNCTION_HARD_LIMIT,
            format!(
                "function `{function_name}` has {line_count} lines; hard limit is {}",
                policy.hard_function_lines
            ),
            "Split the function into responsibility-specific operations.",
        ));
    }
    if line_count >= policy.function_split_lines {
        return Some((
            STRICT_SOURCE_FUNCTION_SPLIT_REQUIRED,
            format!(
                "function `{function_name}` has {line_count} lines; split is required at {}",
                policy.function_split_lines
            ),
            "Extract independent validation or transformation stages into named functions.",
        ));
    }
    (line_count > policy.preferred_function_lines).then_some((
        STRICT_SOURCE_FUNCTION_DECOMPOSITION,
        format!(
            "function `{function_name}` has {line_count} lines; preferred limit is {}",
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
