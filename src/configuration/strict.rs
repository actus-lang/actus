use std::fmt::{Display, Formatter};
use std::path::Path;

use super::{CompilerConfiguration, ConfigurationError, manifest};
use crate::diagnostics::{Diagnostic, DiagnosticPhase, STRICT_MANUAL_FREESTANDING_LINKER};
use crate::lexer::SourceSpan;

#[derive(Debug)]
/// A strict configuration failure carrying a stable diagnostic model. It prints
/// as the same line as the failure outside strict mode, with the same code.
pub struct StrictConfigurationError {
    diagnostic: Box<Diagnostic>,
    report: String,
}

impl StrictConfigurationError {
    /// Returns the renderer-independent diagnostic for this failure.
    pub fn diagnostic(&self) -> &Diagnostic {
        self.diagnostic.as_ref()
    }

    fn from_configuration(error: ConfigurationError) -> Self {
        let diagnostic = Diagnostic::error(error.code(), SourceSpan::new(0, 0), error.to_string())
            .with_phase(DiagnosticPhase::Configuration);
        Self { diagnostic: Box::new(diagnostic), report: error.report() }
    }
}

impl Display for StrictConfigurationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.report)
    }
}

impl std::error::Error for StrictConfigurationError {}

impl CompilerConfiguration {
    pub fn from_manifest_strict(path: &Path) -> Result<Self, StrictConfigurationError> {
        Self::from_manifest_with_lockfile_policy(path, true)
            .and_then(validate_strict_linker)
            .map_err(|error| StrictConfigurationError::from_configuration(error.in_manifest(path)))
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

fn validate_strict_linker(
    configuration: CompilerConfiguration,
) -> Result<CompilerConfiguration, ConfigurationError> {
    if configuration.entry_contract() == crate::configuration::EntryContract::Freestanding
        && configuration.linker_is_explicit()
    {
        let message = format!(
            "manually selected freestanding linker `{}` is rejected in strict mode; use the bundled linker from scripts/bootstrap_llvm.sh or scripts/bootstrap_llvm.ps1",
            configuration.linker().to_string_lossy()
        );
        // The environment variable wins over the manifest key, so only without
        // it is the manifest the place of the failure.
        return Err(if std::env::var_os(super::types::LINKER_ENVIRONMENT_VARIABLE).is_some() {
            ConfigurationError::environment(STRICT_MANUAL_FREESTANDING_LINKER, message)
        } else {
            ConfigurationError::coded(STRICT_MANUAL_FREESTANDING_LINKER, message)
                .at_key("build.linker")
        });
    }
    Ok(configuration)
}
