use std::path::{Path, PathBuf};

use crate::configuration::CompilerConfiguration;

pub(super) fn configuration_for_input(
    input: Option<&str>,
) -> Result<CompilerConfiguration, String> {
    let path = input.map_or_else(
        || {
            std::env::current_dir()
                .map_err(|error| format!("cannot read current directory: {error}"))
        },
        |path| Ok(PathBuf::from(path)),
    )?;
    CompilerConfiguration::from_input_path(&path).map_err(|error| error.to_string())
}

pub(super) fn entry_path(configuration: &CompilerConfiguration, input: Option<&str>) -> PathBuf {
    input.map_or_else(|| configuration.default_entry_path(), PathBuf::from)
}

pub(super) fn source_path(path: &Path) -> String {
    path.display().to_string()
}
