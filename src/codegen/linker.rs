use std::path::Path;
use std::process::Command;

use crate::configuration::CompilerConfiguration;

#[derive(Debug)]
pub struct NativeLinkError(String);

impl std::fmt::Display for NativeLinkError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for NativeLinkError {}

pub fn link_object(
    object: &Path,
    executable: &Path,
    configuration: &CompilerConfiguration,
) -> Result<(), NativeLinkError> {
    let linker = configuration.linker();
    let output = Command::new(linker)
        .arg(object)
        .arg("-o")
        .arg(executable)
        .output()
        .map_err(|error| NativeLinkError(format!("cannot execute linker: {error}")))?;
    if output.status.success() {
        return Ok(());
    }
    let details = String::from_utf8_lossy(&output.stderr);
    Err(NativeLinkError(format!(
        "linker `{}` failed: {}",
        linker.to_string_lossy(),
        details.trim()
    )))
}
