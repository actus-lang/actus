use std::path::{Path, PathBuf};

use crate::configuration::CompilerConfiguration;

use super::conformance::ConformanceMode;

pub(super) fn configuration_for_input(
    input: Option<&str>,
) -> Result<CompilerConfiguration, String> {
    configuration_for_input_with_mode(input, ConformanceMode::Standard)
}

pub(super) fn configuration_for_input_with_mode(
    input: Option<&str>,
    mode: ConformanceMode,
) -> Result<CompilerConfiguration, String> {
    let path = input.map_or_else(
        || {
            std::env::current_dir()
                .map_err(|error| format!("cannot read current directory: {error}"))
        },
        |path| Ok(PathBuf::from(path)),
    )?;
    let configuration = if mode.is_strict() {
        CompilerConfiguration::from_input_path_strict(&path)
    } else {
        CompilerConfiguration::from_input_path(&path)
    };
    configuration.map_err(|error| error.to_string())
}

pub(super) fn entry_path(configuration: &CompilerConfiguration, input: Option<&str>) -> PathBuf {
    input.map_or_else(|| configuration.default_entry_path(), PathBuf::from)
}

pub(super) fn source_path(path: &Path) -> String {
    path.display().to_string()
}

pub(super) fn module_path_for_file(source_root: &Path, file: &Path) -> Option<String> {
    let mut directory = file.parent()?;
    while directory.starts_with(source_root) && directory != source_root {
        let name = directory.file_name()?.to_str()?;
        if directory.join(format!("{name}.act")).is_file() {
            let relative = directory.strip_prefix(source_root).ok()?;
            let segments = relative
                .components()
                .map(|component| component.as_os_str().to_str())
                .collect::<Option<Vec<_>>>()?;
            return Some(segments.join("::"));
        }
        directory = directory.parent()?;
    }
    None
}
