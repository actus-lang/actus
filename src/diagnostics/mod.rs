mod model;
mod renderer;
mod strict_codes;

pub use model::{Diagnostic, DiagnosticPhase, DiagnosticSeverity, sort_diagnostics};
pub use renderer::{
    lex_diagnostic, parse_diagnostic, render_colored_diagnostic, render_diagnostic,
    render_json_diagnostics, render_lex_error, render_parse_error, render_semantic_error,
    semantic_diagnostic,
};
pub use strict_codes::{
    STRICT_CODE_RANGE_END, STRICT_CODE_RANGE_START, STRICT_CONFIGURATION_FAILURE,
    STRICT_LEGACY_DEPENDENCY, STRICT_LEGACY_MANIFEST, StrictDiagnosticCategory,
    strict_code_category,
};
