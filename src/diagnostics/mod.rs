mod model;
mod renderer;
mod strict_codes;

pub use model::{
    Diagnostic, DiagnosticCatalogError, DiagnosticDefinition, DiagnosticPhase,
    DiagnosticRelatedLocation, DiagnosticSeverity, sort_diagnostics, validate_diagnostic_catalog,
};
pub use renderer::{
    lex_diagnostic, module_diagnostic, parse_diagnostic, render_colored_diagnostic,
    render_diagnostic, render_json_diagnostics, render_lex_error, render_parse_error,
    render_semantic_error, semantic_diagnostic,
};
pub use strict_codes::{
    STRICT_ACTUS_FIXTURE_RUST_SOURCE, STRICT_CODE_RANGE_END, STRICT_CODE_RANGE_START,
    STRICT_CONFIGURATION_FAILURE, STRICT_DOCUMENTATION_CONTRADICTION,
    STRICT_DOCUMENTATION_MISREPRESENTATION, STRICT_DOCUMENTATION_MISSING,
    STRICT_DOCUMENTATION_RESTATEMENT, STRICT_DOCUMENTATION_SECTION, STRICT_LEGACY_DEPENDENCY,
    STRICT_LEGACY_MANIFEST, STRICT_RUNTIME_METADATA_MISSING, STRICT_RUNTIME_TARGET_INCOMPATIBLE,
    STRICT_RUNTIME_UNSUPPORTED, STRICT_SOURCE_FILE_DECOMPOSITION, STRICT_SOURCE_FILE_HARD_LIMIT,
    STRICT_SOURCE_FILE_SPLIT_REQUIRED, STRICT_SOURCE_FUNCTION_DECOMPOSITION,
    STRICT_SOURCE_FUNCTION_HARD_LIMIT, STRICT_SOURCE_FUNCTION_SPLIT_REQUIRED,
    STRICT_SOURCE_LIMIT_SUPPRESSION, StrictDiagnosticCategory, strict_code_category,
};
