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
    link_objects(&[object], executable, configuration)
}

pub fn link_objects(
    objects: &[&Path],
    executable: &Path,
    configuration: &CompilerConfiguration,
) -> Result<(), NativeLinkError> {
    if objects.is_empty() {
        return Err(NativeLinkError("link requires at least one object".to_owned()));
    }
    let linker = configuration.linker();
    let mut command = configure_link_command(linker, objects, configuration);
    append_link_inputs(&mut command, configuration);
    let output = run_link_command(&mut command, executable, configuration)?;
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

fn configure_link_command(
    linker: &std::ffi::OsStr,
    objects: &[&Path],
    configuration: &CompilerConfiguration,
) -> Command {
    let mut command = Command::new(linker);
    if configuration.linker_flavor() == crate::configuration::LinkerFlavor::Msvc
        && is_rust_lld(linker)
    {
        command.args(["-flavor", "link"]);
    }
    append_object_inputs(&mut command, objects);
    command
}

fn append_object_inputs(command: &mut Command, objects: &[&Path]) {
    for object in objects {
        command.arg(object);
    }
}

fn append_link_inputs(command: &mut Command, configuration: &CompilerConfiguration) {
    append_runtime_inputs(command, configuration);
    for path in configuration.library_paths() {
        command.arg(configuration.linker_flavor().library_path_argument(path));
    }
    for library in configuration.libraries() {
        command.args(configuration.linker_flavor().library_arguments(
            library.name(),
            matches!(library.kind(), crate::configuration::LibraryKind::Static),
        ));
    }
}

fn append_runtime_inputs(command: &mut Command, configuration: &CompilerConfiguration) {
    if configuration.host_runtime_enabled()
        && let Some(runtime_archive) = crate::runtime::runtime_archive_path()
    {
        command.arg(runtime_archive);
        command.args(configuration.linker_flavor().runtime_library_arguments());
    }
}

fn run_link_command(
    command: &mut Command,
    executable: &Path,
    configuration: &CompilerConfiguration,
) -> Result<std::process::Output, NativeLinkError> {
    command
        .args(configuration.linker_flavor().output_arguments(executable))
        .output()
        .map_err(|error| NativeLinkError(format!("cannot execute linker: {error}")))
}

fn is_rust_lld(linker: &std::ffi::OsStr) -> bool {
    linker
        .to_string_lossy()
        .rsplit(['/', '\\'])
        .next()
        .and_then(|name| name.strip_suffix(".exe").or(Some(name)))
        .is_some_and(|name| name.eq_ignore_ascii_case("rust-lld"))
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
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

    #[test]
    fn emits_platform_specific_output_arguments() {
        let path = Path::new("/tmp/actus-output");
        assert_eq!(LinkerFlavor::Gnu.output_arguments(path), ["-o", "/tmp/actus-output"]);
        assert_eq!(LinkerFlavor::Apple.output_arguments(path), ["-o", "/tmp/actus-output"]);
        assert_eq!(LinkerFlavor::Msvc.output_arguments(path), ["/OUT:/tmp/actus-output"]);
    }

    #[test]
    fn emits_msvc_runtime_library_arguments() {
        let libraries = LinkerFlavor::Msvc.runtime_library_arguments();
        assert!(libraries.contains(&"kernel32.lib"));
        assert!(libraries.contains(&"libcmt.lib"));
        assert!(libraries.contains(&"ws2_32.lib"));
        assert!(LinkerFlavor::Gnu.runtime_library_arguments().is_empty());
    }

    #[test]
    fn identifies_the_rust_lld_driver() {
        assert!(super::is_rust_lld(OsStr::new("rust-lld.exe")));
        assert!(super::is_rust_lld(OsStr::new("C:\\Rust\\rust-lld.exe")));
        assert!(!super::is_rust_lld(OsStr::new("lld-link.exe")));
    }

    #[test]
    fn appends_each_module_object_to_one_link_command() {
        let mut command = std::process::Command::new("linker");
        let first = Path::new("/tmp/root.o");
        let second = Path::new("/tmp/module.o");
        super::append_object_inputs(&mut command, &[first, second]);
        let arguments = command
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(arguments, ["/tmp/root.o", "/tmp/module.o"]);
    }
}
