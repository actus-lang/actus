use std::fmt::{Display, Formatter};
use std::path::Path;

use super::{CompilerConfiguration, ConfigurationError, manifest};
use crate::diagnostics::{
    Diagnostic, DiagnosticPhase, STRICT_CONFIGURATION_FAILURE, STRICT_RUNTIME_METADATA_MISSING,
    STRICT_RUNTIME_TARGET_INCOMPATIBLE, STRICT_RUNTIME_UNSUPPORTED,
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
        let code = if message.contains("cannot locate the compiler-owned standard library")
            || message.contains("builtin standard library is missing its runtime registry")
            || message.contains("builtin standard-library root has no package parent")
        {
            STRICT_RUNTIME_METADATA_MISSING
        } else if message.contains("unsupported runtime profile") {
            STRICT_RUNTIME_UNSUPPORTED
        } else if message.contains("runtime profile `freestanding` is incompatible") {
            STRICT_RUNTIME_TARGET_INCOMPATIBLE
        } else {
            STRICT_CONFIGURATION_FAILURE
        };
        Self {
            diagnostic: Box::new(
                Diagnostic::error(code, SourceSpan::new(0, 0), message)
                    .with_phase(DiagnosticPhase::Configuration),
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
        Self::from_manifest_with_lockfile_policy(path, true)
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
