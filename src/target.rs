use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::Deserialize;
use target_lexicon::{
    Architecture, ArmArchitecture, BinaryFormat, CallingConvention, Endianness, Environment,
    OperatingSystem, PointerWidth, Triple,
};

mod counter;
mod provider;

pub use counter::{CounterExtender, CounterSampleError};
pub use provider::{
    CalibrationPolicy, ClockUnit, DiscontinuityPolicy, Initialization, InterruptSafety,
    ProviderContractError, ProviderConversionError, ReadAtomicity, ResetPolicy, SleepPolicy,
    TimeProviderContract, TimeProviderManifest, WrapBehavior,
};

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
pub enum ArmInstructionMode {
    Thumb2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Arm32TargetProfile {
    CortexMv7em,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TargetSpec {
    triple: Triple,
    pub architecture: Architecture,
    pub pointer_width: PointerWidth,
    pub endianness: Endianness,
    pub object_format: BinaryFormat,
    pub abi: CallingConvention,
    linker_flavor: LinkerFlavor,
}

#[derive(Debug, Eq, PartialEq)]
pub struct TargetSpecError(String);

impl From<String> for TargetSpecError {
    fn from(message: String) -> Self {
        Self(message)
    }
}

impl Display for TargetSpecError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for TargetSpecError {}

impl TargetSpec {
    pub fn host() -> Result<Self, TargetSpecError> {
        Self::from_triple(Triple::host())
    }

    pub fn from_triple(triple: Triple) -> Result<Self, TargetSpecError> {
        let pointer_width = triple.pointer_width().map_err(|()| {
            TargetSpecError(format!("target `{triple}` has no supported pointer width"))
        })?;
        let endianness = triple.endianness().map_err(|()| {
            TargetSpecError(format!("target `{triple}` has no supported endianness"))
        })?;
        let abi = triple
            .default_calling_convention()
            .or_else(|()| {
                if is_freestanding_target(&triple) {
                    Ok(CallingConvention::SystemV)
                } else {
                    Err(())
                }
            })
            .map_err(|()| TargetSpecError(format!("target `{triple}` has no default ABI")))?;
        let linker_flavor = LinkerFlavor::from_target(&triple)?;
        Ok(Self {
            architecture: triple.architecture,
            pointer_width,
            endianness,
            object_format: triple.binary_format,
            abi,
            linker_flavor,
            triple,
        })
    }

    pub fn parse(value: &str) -> Result<Self, TargetSpecError> {
        if value == "host" {
            return Self::host();
        }
        let triple = Triple::from_str(value)
            .map_err(|error| TargetSpecError(format!("invalid target `{value}`: {error}")))?;
        Self::from_triple(triple)
    }

    pub fn triple(&self) -> &Triple {
        &self.triple
    }

    pub const fn arm32_profile(&self) -> Option<Arm32TargetProfile> {
        match self.architecture {
            Architecture::Arm(ArmArchitecture::Thumbv7em)
                if matches!(self.triple.operating_system, OperatingSystem::None_) =>
            {
                Some(Arm32TargetProfile::CortexMv7em)
            }
            _ => None,
        }
    }

    pub const fn arm_instruction_mode(&self) -> Option<ArmInstructionMode> {
        match self.arm32_profile() {
            Some(_) => Some(ArmInstructionMode::Thumb2),
            None => None,
        }
    }

    pub fn matches_platform(&self, selector: &str) -> bool {
        match selector {
            "windows" => self.triple.operating_system == OperatingSystem::Windows,
            "unix" | "posix" => {
                self.triple.operating_system != OperatingSystem::Windows
                    && self.entry_contract() == EntryContract::Hosted
            }
            _ => false,
        }
    }

    pub const fn linker_flavor(&self) -> LinkerFlavor {
        self.linker_flavor
    }

    pub fn default_linker(&self) -> &'static str {
        self.linker_flavor.default_executable()
    }

    pub fn entry_contract(&self) -> EntryContract {
        match self.triple.operating_system {
            OperatingSystem::Unknown
            | OperatingSystem::None_
            | OperatingSystem::Uefi
            | OperatingSystem::Espidf
            | OperatingSystem::VxWorks => EntryContract::Freestanding,
            _ => EntryContract::Hosted,
        }
    }

    pub fn artifact_contract(&self) -> TargetArtifactContract {
        let entry_contract = self.entry_contract();
        TargetArtifactContract {
            object_format: self.object_format,
            linker_flavor: self.linker_flavor,
            entry_contract,
            startup: match entry_contract {
                EntryContract::Hosted => StartupContract::HostedRuntime,
                EntryContract::Freestanding => StartupContract::ExplicitEntry,
            },
            entry_visibility: SymbolVisibility::Exported,
            runtime_visibility: SymbolVisibility::Imported,
        }
    }

    pub fn spec_hash(&self) -> String {
        let identity = format!(
            "triple={};architecture={:?};pointer_width={:?};endianness={:?};object_format={:?};abi={:?};linker={:?}",
            self.triple,
            self.architecture,
            self.pointer_width,
            self.endianness,
            self.object_format,
            self.abi,
            self.linker_flavor
        );
        let mut hash = 0xcbf29ce484222325_u64;
        for byte in identity.bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{hash:016x}")
    }
}

fn is_freestanding_target(triple: &Triple) -> bool {
    matches!(
        triple.operating_system,
        OperatingSystem::Unknown
            | OperatingSystem::None_
            | OperatingSystem::Uefi
            | OperatingSystem::Espidf
            | OperatingSystem::VxWorks
    )
}

#[cfg(test)]
mod tests {
    use target_lexicon::{CallingConvention, Endianness, PointerWidth};

    use super::{EntryContract, LinkerFlavor, StartupContract, TargetSpec};

    #[test]
    fn derives_target_contract_fields_from_a_triple() {
        let spec = TargetSpec::parse("x86_64-unknown-linux-gnu").expect("target should parse");
        assert_eq!(spec.pointer_width, PointerWidth::U64);
        assert_eq!(spec.endianness, Endianness::Little);
        assert_eq!(spec.object_format.into_str(), "elf");
        assert_eq!(spec.abi, CallingConvention::SystemV);
        assert_eq!(spec.linker_flavor(), LinkerFlavor::Gnu);
        assert_eq!(spec.default_linker(), "cc");
        assert_eq!(spec.entry_contract(), EntryContract::Hosted);
        assert_eq!(spec.artifact_contract().startup, StartupContract::HostedRuntime);
        assert_eq!(spec.triple().to_string(), "x86_64-unknown-linux-gnu");
    }

    #[test]
    fn derives_explicit_startup_for_freestanding_targets() {
        let spec = TargetSpec::parse("thumbv7em-none-eabihf").expect("target should parse");
        let contract = spec.artifact_contract();
        assert_eq!(contract.entry_contract, EntryContract::Freestanding);
        assert_eq!(contract.startup, StartupContract::ExplicitEntry);
    }

    #[test]
    fn derives_linker_flavor_from_target_environment() {
        assert_eq!(
            TargetSpec::parse("x86_64-apple-darwin")
                .expect("Apple target should parse")
                .linker_flavor(),
            LinkerFlavor::Apple
        );
        assert_eq!(
            TargetSpec::parse("x86_64-pc-windows-msvc")
                .expect("MSVC target should parse")
                .linker_flavor(),
            LinkerFlavor::Msvc
        );
        assert_eq!(
            TargetSpec::parse("x86_64-pc-windows-gnu")
                .expect("GNU Windows target should parse")
                .linker_flavor(),
            LinkerFlavor::Gnu
        );
        assert_eq!(
            TargetSpec::parse("x86_64-pc-windows-msvc")
                .expect("MSVC target should parse")
                .default_linker(),
            "lld-link.exe"
        );
    }

    #[test]
    fn rejects_targets_without_a_supported_address_width() {
        let triple = "unknown-unknown-unknown"
            .parse()
            .expect("unknown target triple should parse structurally");
        assert!(TargetSpec::from_triple(triple).is_err());
    }
}
