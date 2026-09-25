use crate::diagnostics::{render_lex_error, render_parse_error, render_semantic_error};
use crate::lexer::scan;
use crate::parser::parse;
use crate::semantic::analyze;

use super::position::{LineIndex, LspRange};
use super::protocol::LspDiagnostic;

pub fn analyze_document(source: &str) -> Vec<LspDiagnostic> {
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
    if let Err(error) = analyze(&program) {
        return vec![diagnostic(source, &index, error.span, render_semantic_error(source, &error))];
    }
    Vec::new()
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
