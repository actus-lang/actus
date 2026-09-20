use std::path::Path;
use std::process::Command;

#[derive(Debug)]
pub struct NativeLinkError(String);

impl std::fmt::Display for NativeLinkError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for NativeLinkError {}

pub fn link_object(object: &Path, executable: &Path) -> Result<(), NativeLinkError> {
    let linker = std::env::var_os("ACTUS_LINKER").unwrap_or_else(|| "cc".into());
    let output = Command::new(&linker)
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
