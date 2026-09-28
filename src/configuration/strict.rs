use std::path::Path;

use super::{CompilerConfiguration, ConfigurationError, manifest};

impl CompilerConfiguration {
    pub fn from_manifest_strict(path: &Path) -> Result<Self, ConfigurationError> {
        Self::from_manifest_with_lockfile_policy(path, true, true)
    }

    pub fn from_current_manifest_strict() -> Result<Self, ConfigurationError> {
        let Some(path) = manifest::manifest_in_directory(Path::new(".")) else {
            return Ok(Self::from_environment());
        };
        Self::from_manifest_strict(&path)
    }

    pub fn from_input_path_strict(path: &Path) -> Result<Self, ConfigurationError> {
        let start = if path.is_dir() { path } else { path.parent().unwrap_or(path) };
        let Some(manifest) = manifest::find_manifest(start) else {
            return Ok(Self::from_environment());
        };
        Self::from_manifest_strict(&manifest)
    }
}
