use std::path::Path;
use std::process::Command;

use crate::configuration::{CompilerConfiguration, LibraryKind, LinkLibrary, LinkerFlavor};

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
        command.arg(library_path_argument(path, configuration.linker_flavor()));
    }
    for library in configuration.libraries() {
        let arguments = library_arguments(library, configuration.linker_flavor())?;
        command.args(arguments);
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

fn library_path_argument(path: &Path, flavor: LinkerFlavor) -> String {
    match flavor {
        LinkerFlavor::Msvc => format!("/LIBPATH:{}", path.display()),
        LinkerFlavor::Gnu | LinkerFlavor::Apple => format!("-L{}", path.display()),
    }
}

fn library_arguments(
    library: &LinkLibrary,
    flavor: LinkerFlavor,
) -> Result<Vec<String>, NativeLinkError> {
    let name = library.name();
    match (flavor, library.kind()) {
        (LinkerFlavor::Gnu, LibraryKind::Static) => {
            Ok(vec!["-Wl,-Bstatic".to_owned(), format!("-l{name}"), "-Wl,-Bdynamic".to_owned()])
        }
        (LinkerFlavor::Gnu, LibraryKind::Shared) => Ok(vec![format!("-l{name}")]),
        (LinkerFlavor::Apple, LibraryKind::Static) => {
            Ok(vec![format!("-Wl,-force_load,lib{name}.a")])
        }
        (LinkerFlavor::Apple, LibraryKind::Shared) => Ok(vec![format!("-l{name}")]),
        (LinkerFlavor::Msvc, _) => Ok(vec![format!("{name}.lib")]),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{library_arguments, library_path_argument};
    use crate::configuration::{LibraryKind, LinkLibrary, LinkerFlavor};

    fn library(name: &str, kind: LibraryKind) -> LinkLibrary {
        LinkLibrary::new(name.to_owned(), kind)
    }

    #[test]
    fn isolates_gnu_static_flags() {
        assert_eq!(
            library_arguments(&library("math", LibraryKind::Static), LinkerFlavor::Gnu)
                .expect("GNU arguments should be generated"),
            ["-Wl,-Bstatic", "-lmath", "-Wl,-Bdynamic"]
        );
    }

    #[test]
    fn emits_platform_specific_library_arguments() {
        assert_eq!(
            library_arguments(&library("math", LibraryKind::Static), LinkerFlavor::Apple)
                .expect("Apple arguments should be generated"),
            ["-Wl,-force_load,libmath.a"]
        );
        assert_eq!(
            library_arguments(&library("math", LibraryKind::Shared), LinkerFlavor::Msvc)
                .expect("MSVC arguments should be generated"),
            ["math.lib"]
        );
    }

    #[test]
    fn emits_library_search_path_for_each_linker() {
        let path = Path::new("/tmp/actus-libs");
        assert_eq!(library_path_argument(path, LinkerFlavor::Gnu), "-L/tmp/actus-libs");
        assert_eq!(library_path_argument(path, LinkerFlavor::Apple), "-L/tmp/actus-libs");
        assert_eq!(library_path_argument(path, LinkerFlavor::Msvc), "/LIBPATH:/tmp/actus-libs");
    }
}
