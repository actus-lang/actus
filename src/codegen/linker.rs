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
    let mut command = Command::new(linker);
    command.arg(object);
    if let Some(runtime_archive) = crate::runtime::runtime_archive_path() {
        command.arg(runtime_archive);
    }
    for path in configuration.library_paths() {
        command.arg(configuration.linker_flavor().library_path_argument(path));
    }
    for library in configuration.libraries() {
        command.args(configuration.linker_flavor().library_arguments(
            library.name(),
            matches!(library.kind(), crate::configuration::LibraryKind::Static),
        ));
    }
    let output = command
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::target::LinkerFlavor;

    #[test]
    fn isolates_gnu_static_flags() {
        assert_eq!(
            LinkerFlavor::Gnu.library_arguments("math", true),
            ["-Wl,-Bstatic", "-lmath", "-Wl,-Bdynamic"]
        );
    }

    #[test]
    fn emits_platform_specific_library_arguments() {
        assert_eq!(
            LinkerFlavor::Apple.library_arguments("math", true),
            ["-Wl,-force_load,libmath.a"]
        );
        assert_eq!(LinkerFlavor::Msvc.library_arguments("math", false), ["math.lib"]);
    }

    #[test]
    fn emits_library_search_path_for_each_linker() {
        let path = Path::new("/tmp/actus-libs");
        assert_eq!(LinkerFlavor::Gnu.library_path_argument(path), "-L/tmp/actus-libs");
        assert_eq!(LinkerFlavor::Apple.library_path_argument(path), "-L/tmp/actus-libs");
        assert_eq!(LinkerFlavor::Msvc.library_path_argument(path), "/LIBPATH:/tmp/actus-libs");
    }
}
