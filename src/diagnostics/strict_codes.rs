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

/// The selected runtime profile requires unavailable compiler-owned metadata.
pub const STRICT_RUNTIME_METADATA_MISSING: &str = "E1803";

/// The manifest requested a runtime profile that is not supported by this edition.
pub const STRICT_RUNTIME_UNSUPPORTED: &str = "E1804";

/// A runtime profile is incompatible with the selected target contract.
pub const STRICT_RUNTIME_TARGET_INCOMPATIBLE: &str = "E1805";

/// Missing or empty public block documentation.
pub const STRICT_DOCUMENTATION_MISSING: &str = "E1840";

/// A required public documentation contract section is absent.
pub const STRICT_DOCUMENTATION_SECTION: &str = "E1841";

/// Public documentation is an incomplete or signature-restating summary.
pub const STRICT_DOCUMENTATION_RESTATEMENT: &str = "E1842";

/// Public documentation contradicts an ownership contract.
pub const STRICT_DOCUMENTATION_CONTRADICTION: &str = "E1844";

/// Public documentation promises behavior the declaration cannot guarantee.
pub const STRICT_DOCUMENTATION_MISREPRESENTATION: &str = "E1845";

/// A source file exceeded the preferred size and needs decomposition evidence.
pub const STRICT_SOURCE_FILE_DECOMPOSITION: &str = "E1850";

/// A source file reached the strict split threshold.
pub const STRICT_SOURCE_FILE_SPLIT_REQUIRED: &str = "E1851";

/// A source file exceeded the hard size limit.
pub const STRICT_SOURCE_FILE_HARD_LIMIT: &str = "E1852";

/// A function exceeded the preferred size and needs decomposition evidence.
pub const STRICT_SOURCE_FUNCTION_DECOMPOSITION: &str = "E1853";

/// A function reached the strict split threshold.
pub const STRICT_SOURCE_FUNCTION_SPLIT_REQUIRED: &str = "E1854";

/// A function exceeded the hard size limit.
pub const STRICT_SOURCE_FUNCTION_HARD_LIMIT: &str = "E1855";

/// A source-local annotation attempted to suppress an architectural limit.
pub const STRICT_SOURCE_LIMIT_SUPPRESSION: &str = "E1856";

/// Rust source was found inside the Actus standard-library fixture tree.
pub const STRICT_ACTUS_FIXTURE_RUST_SOURCE: &str = "E1834";

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
