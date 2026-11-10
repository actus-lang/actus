use serde::Deserialize;
use target_lexicon::{BinaryFormat, Environment, Triple};

use super::TargetSpecError;
use super::resolution::is_freestanding_target;

#[derive(Clone, Copy, Deserialize, Debug, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LinkerFlavor {
    Gnu,
    Apple,
    Msvc,
}

#[derive(Clone, Copy, Deserialize, Debug, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EntryContract {
    Hosted,
    Freestanding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartupContract {
    HostedRuntime,
    ExplicitEntry,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeProviderContract {
    HostedPlatform,
    ExplicitFreestanding,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FreestandingLinkerContract {
    pub entry_symbol_required: bool,
    pub linker_script_required: bool,
    pub runtime_provider: RuntimeProviderContract,
    pub stack_alignment_bytes: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelocationModel {
    Static,
    PositionIndependent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SymbolVisibility {
    Exported,
    Imported,
    Hidden,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TargetArtifactContract {
    pub object_format: BinaryFormat,
    pub linker_flavor: LinkerFlavor,
    pub entry_contract: EntryContract,
    pub startup: StartupContract,
    pub entry_visibility: SymbolVisibility,
    pub runtime_visibility: SymbolVisibility,
}

impl LinkerFlavor {
    pub fn from_target(triple: &Triple) -> Result<Self, TargetSpecError> {
        if is_freestanding_target(triple) {
            return Ok(Self::Gnu);
        }
        let flavor = match triple.environment {
            Environment::Msvc => Self::Msvc,
            Environment::Gnu
            | Environment::Gnuabi64
            | Environment::Gnueabi
            | Environment::Gnueabihf
            | Environment::Gnuspe
            | Environment::Gnux32
            | Environment::GnuIlp32
            | Environment::GnuLlvm
            | Environment::Musl
            | Environment::Musleabi
            | Environment::Musleabihf
            | Environment::Muslabi64 => Self::Gnu,
            _ => match triple.binary_format {
                BinaryFormat::Macho => Self::Apple,
                BinaryFormat::Coff => Self::Msvc,
                BinaryFormat::Elf | BinaryFormat::Xcoff => Self::Gnu,
                _ => {
                    return Err(TargetSpecError(format!(
                        "target `{triple}` has no supported linker flavor"
                    )));
                }
            },
        };
        Ok(flavor)
    }

    pub fn library_path_argument(self, path: &std::path::Path) -> String {
        match self {
            Self::Msvc => format!("/LIBPATH:{}", path.display()),
            Self::Gnu | Self::Apple => format!("-L{}", path.display()),
        }
    }

    pub fn library_arguments(self, name: &str, static_library: bool) -> Vec<String> {
        match (self, static_library) {
            (Self::Gnu, true) => {
                vec!["-Wl,-Bstatic".to_owned(), format!("-l{name}"), "-Wl,-Bdynamic".to_owned()]
            }
            (Self::Gnu, false) | (Self::Apple, false) => vec![format!("-l{name}")],
            (Self::Apple, true) => vec![format!("-Wl,-force_load,lib{name}.a")],
            (Self::Msvc, _) => vec![format!("{name}.lib")],
        }
    }

    pub fn runtime_library_arguments(self) -> &'static [&'static str] {
        match self {
            Self::Msvc => &[
                "advapi32.lib",
                "bcrypt.lib",
                "kernel32.lib",
                "libcmt.lib",
                "ntdll.lib",
                "oldnames.lib",
                "userenv.lib",
                "ws2_32.lib",
            ],
            Self::Gnu | Self::Apple => &[],
        }
    }

    pub fn output_arguments(self, path: &std::path::Path) -> Vec<String> {
        match self {
            Self::Msvc => vec![format!("/OUT:{}", path.display())],
            Self::Gnu | Self::Apple => vec!["-o".to_owned(), path.display().to_string()],
        }
    }

    pub const fn default_executable(self) -> &'static str {
        match self {
            Self::Gnu | Self::Apple => "cc",
            Self::Msvc => "lld-link.exe",
        }
    }
}
