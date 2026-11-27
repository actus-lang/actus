use super::DiagnosticCode;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// The reserved strict-conformance diagnostic category for an `E18xx` code.
pub enum StrictDiagnosticCategory {
    /// Configuration, manifest, workspace, or policy failures.
    Configuration,
    /// Unsupported, incomplete, or recovered frontend declarations.
    Frontend,
    /// Strict semantic, ownership, type, or exhaustiveness failures.
    Semantic,
    /// Bundled toolchain, linker, and freestanding component failures.
    BundledToolchain,
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
pub const STRICT_CONFIGURATION_FAILURE: &str =
    DiagnosticCode::STRICT_CONFIGURATION_FAILURE.as_str();

/// A package manifest name is not a valid, unique identity namespace.
pub const STRICT_PACKAGE_IDENTITY_INVALID: &str =
    DiagnosticCode::STRICT_PACKAGE_IDENTITY_INVALID.as_str();

/// An LLVM strict build selected an unverified toolchain override.
pub const STRICT_LLVM_UNVERIFIED: &str = DiagnosticCode::STRICT_LLVM_UNVERIFIED.as_str();

/// A required bundled linker or compiler-rt component is missing.
pub const STRICT_BUNDLED_COMPONENT_MISSING: &str =
    DiagnosticCode::STRICT_BUNDLED_COMPONENT_MISSING.as_str();

/// The selected linker does not match the bundled target identity.
pub const STRICT_LINKER_IDENTITY_MISMATCH: &str =
    DiagnosticCode::STRICT_LINKER_IDENTITY_MISMATCH.as_str();

/// Strict mode rejects a manually selected freestanding linker.
pub const STRICT_MANUAL_FREESTANDING_LINKER: &str =
    DiagnosticCode::STRICT_MANUAL_FREESTANDING_LINKER.as_str();

/// The selected runtime profile requires unavailable compiler-owned metadata.
pub const STRICT_RUNTIME_METADATA_MISSING: &str =
    DiagnosticCode::STRICT_RUNTIME_METADATA_MISSING.as_str();

/// The manifest requested a runtime profile that is not supported by this edition.
pub const STRICT_RUNTIME_UNSUPPORTED: &str = DiagnosticCode::STRICT_RUNTIME_UNSUPPORTED.as_str();

/// A runtime profile is incompatible with the selected target contract.
pub const STRICT_RUNTIME_TARGET_INCOMPATIBLE: &str =
    DiagnosticCode::STRICT_RUNTIME_TARGET_INCOMPATIBLE.as_str();

/// Missing or empty public block documentation.
pub const STRICT_DOCUMENTATION_MISSING: &str =
    DiagnosticCode::STRICT_DOCUMENTATION_MISSING.as_str();

/// A required public documentation contract section is absent.
pub const STRICT_DOCUMENTATION_SECTION: &str =
    DiagnosticCode::STRICT_DOCUMENTATION_SECTION.as_str();

/// Public documentation is an incomplete or signature-restating summary.
pub const STRICT_DOCUMENTATION_RESTATEMENT: &str =
    DiagnosticCode::STRICT_DOCUMENTATION_RESTATEMENT.as_str();

/// Public documentation contradicts an ownership contract.
pub const STRICT_DOCUMENTATION_CONTRADICTION: &str =
    DiagnosticCode::STRICT_DOCUMENTATION_CONTRADICTION.as_str();

/// Public documentation promises behavior the declaration cannot guarantee.
pub const STRICT_DOCUMENTATION_MISREPRESENTATION: &str =
    DiagnosticCode::STRICT_DOCUMENTATION_MISREPRESENTATION.as_str();

/// A source file exceeded the preferred size and needs decomposition evidence.
pub const STRICT_SOURCE_FILE_DECOMPOSITION: &str =
    DiagnosticCode::STRICT_SOURCE_FILE_DECOMPOSITION.as_str();

/// A source file reached the strict split threshold.
pub const STRICT_SOURCE_FILE_SPLIT_REQUIRED: &str =
    DiagnosticCode::STRICT_SOURCE_FILE_SPLIT_REQUIRED.as_str();

/// A source file exceeded the hard size limit.
pub const STRICT_SOURCE_FILE_HARD_LIMIT: &str =
    DiagnosticCode::STRICT_SOURCE_FILE_HARD_LIMIT.as_str();

/// A function exceeded the preferred size and needs decomposition evidence.
pub const STRICT_SOURCE_FUNCTION_DECOMPOSITION: &str =
    DiagnosticCode::STRICT_SOURCE_FUNCTION_DECOMPOSITION.as_str();

/// A function reached the strict split threshold.
pub const STRICT_SOURCE_FUNCTION_SPLIT_REQUIRED: &str =
    DiagnosticCode::STRICT_SOURCE_FUNCTION_SPLIT_REQUIRED.as_str();

/// A function exceeded the hard size limit.
pub const STRICT_SOURCE_FUNCTION_HARD_LIMIT: &str =
    DiagnosticCode::STRICT_SOURCE_FUNCTION_HARD_LIMIT.as_str();

/// A source-local annotation attempted to suppress an architectural limit.
pub const STRICT_SOURCE_LIMIT_SUPPRESSION: &str =
    DiagnosticCode::STRICT_SOURCE_LIMIT_SUPPRESSION.as_str();

/// Rust source was found inside the Actus standard-library fixture tree.
pub const STRICT_ACTUS_FIXTURE_RUST_SOURCE: &str =
    DiagnosticCode::STRICT_ACTUS_FIXTURE_RUST_SOURCE.as_str();

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
        1860..=1879 => StrictDiagnosticCategory::Execution,
        1880..=1889 => StrictDiagnosticCategory::BundledToolchain,
        1890..=1899 => StrictDiagnosticCategory::Execution,
        _ => unreachable!("strict code range was checked above"),
    })
}
