mod model;
mod renderer;

pub use model::{Diagnostic, DiagnosticSeverity};
pub use renderer::{
    lex_diagnostic, parse_diagnostic, render_diagnostic, render_lex_error, render_parse_error,
    render_semantic_error, semantic_diagnostic,
};
