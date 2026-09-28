mod model;
mod renderer;
mod strict_codes;

pub use model::{
    Diagnostic, DiagnosticCatalogError, DiagnosticDefinition, DiagnosticPhase, DiagnosticSeverity,
    sort_diagnostics, validate_diagnostic_catalog,
};
pub use renderer::{
    lex_diagnostic, module_diagnostic, parse_diagnostic, render_colored_diagnostic,
    render_diagnostic, render_json_diagnostics, render_lex_error, render_parse_error,
    render_semantic_error, semantic_diagnostic,
};
pub use strict_codes::{
    STRICT_CODE_RANGE_END, STRICT_CODE_RANGE_START, STRICT_CONFIGURATION_FAILURE,
    STRICT_DOCUMENTATION_CONTRADICTION, STRICT_DOCUMENTATION_MISREPRESENTATION,
    STRICT_DOCUMENTATION_MISSING, STRICT_DOCUMENTATION_RESTATEMENT, STRICT_DOCUMENTATION_SECTION,
    STRICT_LEGACY_DEPENDENCY, STRICT_LEGACY_MANIFEST, STRICT_SOURCE_FILE_DECOMPOSITION,
    STRICT_SOURCE_FILE_HARD_LIMIT, STRICT_SOURCE_FILE_SPLIT_REQUIRED,
    STRICT_SOURCE_FUNCTION_DECOMPOSITION, STRICT_SOURCE_FUNCTION_HARD_LIMIT,
    STRICT_SOURCE_FUNCTION_SPLIT_REQUIRED, StrictDiagnosticCategory, strict_code_category,
};
