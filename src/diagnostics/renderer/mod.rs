mod codes;
mod json;
mod module;
mod render;

use super::model::{Diagnostic, DiagnosticPhase, DiagnosticSeverity};

pub use json::render_json_diagnostics;
pub use module::module_diagnostic;
pub use render::{
    lex_diagnostic, parse_diagnostic, render_colored_diagnostic, render_diagnostic,
    render_lex_error, render_parse_error, render_semantic_error, semantic_diagnostic,
};
