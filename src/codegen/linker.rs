use std::ffi::{OsStr, OsString};
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinkerPlan {
    linker: OsString,
    arguments: Vec<OsString>,
}

impl LinkerPlan {
    pub fn from_configuration(
        objects: &[&Path],
        executable: &Path,
        configuration: &CompilerConfiguration,
    ) -> Result<Self, NativeLinkError> {
        if objects.is_empty() {
            return Err(NativeLinkError("link requires at least one object".to_owned()));
        }
        let linker = configuration.linker().to_os_string();
        let mut arguments = Vec::new();
        if configuration.linker_flavor() == crate::configuration::LinkerFlavor::Msvc
            && is_rust_lld(&linker)
        {
            arguments.extend([OsString::from("-flavor"), OsString::from("link")]);
        }
        if let Some(linker_script) = configuration.linker_script() {
            if configuration.linker_flavor() != crate::configuration::LinkerFlavor::Gnu {
                return Err(NativeLinkError(
                    "linker_script is currently supported only by the GNU linker flavor".to_owned(),
                ));
            }
            arguments.extend([OsString::from("-T"), linker_script.as_os_str().to_os_string()]);
        }
        arguments.extend(objects.iter().map(|object| object.as_os_str().to_os_string()));
        append_link_inputs(&mut arguments, configuration);
        arguments.extend(
            configuration
                .linker_flavor()
                .output_arguments(executable)
                .into_iter()
                .map(OsString::from),
        );
        Ok(Self { linker, arguments })
    }

    pub fn linker(&self) -> &OsStr {
        &self.linker
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.linker);
        command.args(&self.arguments);
        command
    }
}

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
    let plan = LinkerPlan::from_configuration(objects, executable, configuration)?;
    let output = run_link_command(&plan)?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let status = output
        .status
        .code()
        .map_or_else(|| "terminated without an exit code".to_owned(), |code| code.to_string());
    let arguments = plan
        .arguments
        .iter()
        .map(|argument| argument.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ");
    Err(NativeLinkError(format!(
        "linker `{}` failed with status {status}: stderr=`{}` stdout=`{}` command=`{} {arguments}`",
        plan.linker.to_string_lossy(),
        stderr.trim(),
        stdout.trim(),
        plan.linker.to_string_lossy()
    )))
}

fn append_link_inputs(arguments: &mut Vec<OsString>, configuration: &CompilerConfiguration) {
    append_runtime_inputs(arguments, configuration);
    for path in configuration.library_paths() {
        arguments.push(OsString::from(configuration.linker_flavor().library_path_argument(path)));
    }
    for library in configuration.libraries() {
        arguments.extend(
            configuration
                .linker_flavor()
                .library_arguments(
                    library.name(),
                    matches!(library.kind(), crate::configuration::LibraryKind::Static),
                )
                .into_iter()
                .map(OsString::from),
        );
    }
}

fn append_runtime_inputs(arguments: &mut Vec<OsString>, configuration: &CompilerConfiguration) {
    if configuration.host_runtime_enabled()
        && let Some(runtime_archive) = crate::runtime::runtime_archive_path()
    {
        arguments.push(runtime_archive.as_os_str().to_os_string());
        arguments.extend(
            configuration.linker_flavor().runtime_library_arguments().iter().map(OsString::from),
        );
    }
}

fn run_link_command(plan: &LinkerPlan) -> Result<std::process::Output, NativeLinkError> {
    let mut command = plan.command();
    command.output().map_err(|error| NativeLinkError(format!("cannot execute linker: {error}")))
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
}
