#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// The reserved strict-conformance diagnostic category for an `E18xx` code.
pub enum StrictDiagnosticCategory {
    /// Configuration, manifest, workspace, or policy failures.
    Configuration,
    /// Unsupported, incomplete, or recovered frontend declarations.
    Frontend,
    /// Strict semantic, ownership, type, or exhaustiveness failures.
    Semantic,
    /// Module, facade, dependency, naming, or responsibility violations.
    Architecture,
    /// Missing or incomplete public documentation contracts.
    Documentation,
    /// Source-limit and structural conformance failures.
    Limits,
    /// Test, artifact, lockfile, or runtime contract failures.
    Execution,
}

/// The first numeric code reserved for strict-conformance diagnostics.
pub const STRICT_CODE_RANGE_START: u16 = 1800;

/// The last numeric code reserved for strict-conformance diagnostics.
pub const STRICT_CODE_RANGE_END: u16 = 1899;

/// Stable diagnostic code for strict configuration failures without a more
/// specific classification.
pub const STRICT_CONFIGURATION_FAILURE: &str = "E1800";

/// Stable diagnostic code for a deprecated root `Arca.toml` manifest rejected
/// by strict configuration loading.
pub const STRICT_LEGACY_MANIFEST: &str = "E1801";

/// Stable diagnostic code for a deprecated local dependency manifest rejected
/// by strict configuration loading.
pub const STRICT_LEGACY_DEPENDENCY: &str = "E1802";

/// Classifies a strict-conformance code by its reserved `E18xx` range.
pub fn strict_code_category(code: &str) -> Option<StrictDiagnosticCategory> {
    let number = code.strip_prefix('E')?.parse::<u16>().ok()?;
    if !(STRICT_CODE_RANGE_START..=STRICT_CODE_RANGE_END).contains(&number) {
        return None;
    }
    Some(match number {
        1800..=1809 => StrictDiagnosticCategory::Configuration,
        1810..=1819 => StrictDiagnosticCategory::Frontend,
        1820..=1829 => StrictDiagnosticCategory::Semantic,
        1830..=1839 => StrictDiagnosticCategory::Architecture,
        1840..=1849 => StrictDiagnosticCategory::Documentation,
        1850..=1859 => StrictDiagnosticCategory::Limits,
        _ => StrictDiagnosticCategory::Execution,
    })
}
