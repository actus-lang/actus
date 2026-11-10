use std::str::FromStr;

use target_lexicon::{Architecture, ArmArchitecture, OperatingSystem, Triple};

use super::{
    Arm32AbiContract, Arm32TargetProfile, ArmInstructionMode, TargetSpecError,
    contracts::{
        EntryContract, FreestandingLinkerContract, LinkerFlavor, RuntimeProviderContract,
        StartupContract, SymbolVisibility, TargetArtifactContract,
    },
    model::TargetSpec,
};

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
                    Ok(target_lexicon::CallingConvention::SystemV)
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

    pub const fn arm32_abi_contract(&self) -> Option<Arm32AbiContract> {
        match self.arm32_profile() {
            Some(Arm32TargetProfile::CortexMv7em) => Some(Arm32AbiContract::cortex_mv7em()),
            None => None,
        }
    }

    pub const fn freestanding_linker_contract(&self) -> Option<FreestandingLinkerContract> {
        match self.arm32_profile() {
            Some(Arm32TargetProfile::CortexMv7em) => Some(FreestandingLinkerContract {
                entry_symbol_required: true,
                linker_script_required: true,
                runtime_provider: RuntimeProviderContract::ExplicitFreestanding,
                stack_alignment_bytes: 8,
            }),
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

    pub fn supports_capability(&self, capability: crate::ast::Capability) -> bool {
        match capability {
            crate::ast::Capability::FilesystemCreateExclusive => {
                self.entry_contract() == EntryContract::Hosted
                    && (self.matches_platform("posix")
                        || self.triple.operating_system == OperatingSystem::Windows)
            }
            crate::ast::Capability::PosixCloexec
            | crate::ast::Capability::PosixCreateExclusive
            | crate::ast::Capability::UnixRenameAtomic => self.matches_platform("posix"),
            crate::ast::Capability::WindowsFileIdentity
            | crate::ast::Capability::WindowsCreateNew
            | crate::ast::Capability::WindowsReplaceFile => {
                self.triple.operating_system == OperatingSystem::Windows
            }
            crate::ast::Capability::VolatileWrite => true,
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

pub(super) fn is_freestanding_target(triple: &Triple) -> bool {
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
