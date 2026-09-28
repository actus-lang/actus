/// Stable diagnostic code for strict configuration failures without a more
/// specific classification.
pub const STRICT_CONFIGURATION_FAILURE: &str = "E1800";

/// Stable diagnostic code for a deprecated root `Arca.toml` manifest rejected
/// by strict configuration loading.
pub const STRICT_LEGACY_MANIFEST: &str = "E1801";

/// Stable diagnostic code for a deprecated local dependency manifest rejected
/// by strict configuration loading.
pub const STRICT_LEGACY_DEPENDENCY: &str = "E1802";
