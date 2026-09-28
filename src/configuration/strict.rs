use std::fmt::{Display, Formatter};
use std::path::Path;

use super::{CompilerConfiguration, ConfigurationError, manifest};
use crate::diagnostics::{
    Diagnostic, STRICT_CONFIGURATION_FAILURE, STRICT_LEGACY_DEPENDENCY, STRICT_LEGACY_MANIFEST,
};
use crate::lexer::SourceSpan;

#[derive(Debug)]
/// A strict configuration failure carrying a stable diagnostic model.
pub struct StrictConfigurationError {
    diagnostic: Box<Diagnostic>,
}

impl StrictConfigurationError {
    /// Returns the renderer-independent diagnostic for this failure.
    pub fn diagnostic(&self) -> &Diagnostic {
        self.diagnostic.as_ref()
    }

    fn from_configuration(error: ConfigurationError) -> Self {
        let message = error.to_string();
        let code = if message.contains("deprecated dependency `Arca.toml`") {
            STRICT_LEGACY_DEPENDENCY
        } else {
            STRICT_CONFIGURATION_FAILURE
        };
        Self { diagnostic: Box::new(Diagnostic::error(code, SourceSpan::new(0, 0), message)) }
    }

    fn legacy_manifest(path: &Path) -> Self {
        Self {
            diagnostic: Box::new(
                Diagnostic::error(
                    STRICT_LEGACY_MANIFEST,
                    SourceSpan::new(0, 0),
                    "strict mode rejects deprecated `Arca.toml`; rename it to `Actus.toml`",
                )
                .with_source_path(path.display().to_string()),
            ),
        }
    }
}

impl Display for StrictConfigurationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(path) = self.diagnostic.source_path() {
            write!(formatter, "{path}: ")?;
        }
        write!(formatter, "{}: {}", self.diagnostic.code(), self.diagnostic.message())
    }
}

impl std::error::Error for StrictConfigurationError {}

impl CompilerConfiguration {
    pub fn from_manifest_strict(path: &Path) -> Result<Self, StrictConfigurationError> {
        if manifest::is_legacy_manifest(path) {
            return Err(StrictConfigurationError::legacy_manifest(path));
        }
        Self::from_manifest_with_lockfile_policy(path, true, true)
            .map_err(StrictConfigurationError::from_configuration)
    }

    pub fn from_current_manifest_strict() -> Result<Self, StrictConfigurationError> {
        let Some(path) = manifest::manifest_in_directory(Path::new(".")) else {
            return Ok(Self::from_environment());
        };
        Self::from_manifest_strict(&path)
    }

    pub fn from_input_path_strict(path: &Path) -> Result<Self, StrictConfigurationError> {
        let start = if path.is_dir() { path } else { path.parent().unwrap_or(path) };
        let Some(manifest) = manifest::find_manifest(start) else {
            return Ok(Self::from_environment());
        };
        Self::from_manifest_strict(&manifest)
    }
}
