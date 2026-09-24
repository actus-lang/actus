use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::Deserialize;
use target_lexicon::{
    Architecture, BinaryFormat, CallingConvention, Endianness, Environment, PointerWidth, Triple,
};

#[derive(Clone, Copy, Deserialize, Debug, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LinkerFlavor {
    Gnu,
    Apple,
    Msvc,
}

impl LinkerFlavor {
    pub fn from_target(triple: &Triple) -> Result<Self, TargetSpecError> {
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

    pub const fn linker_flavor(&self) -> LinkerFlavor {
        self.linker_flavor
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

#[cfg(test)]
mod tests {
    use target_lexicon::{CallingConvention, Endianness, PointerWidth};

    use super::{LinkerFlavor, TargetSpec};

    #[test]
    fn derives_target_contract_fields_from_a_triple() {
        let spec = TargetSpec::parse("x86_64-unknown-linux-gnu").expect("target should parse");
        assert_eq!(spec.pointer_width, PointerWidth::U64);
        assert_eq!(spec.endianness, Endianness::Little);
        assert_eq!(spec.object_format.into_str(), "elf");
        assert_eq!(spec.abi, CallingConvention::SystemV);
        assert_eq!(spec.linker_flavor(), LinkerFlavor::Gnu);
        assert_eq!(spec.triple().to_string(), "x86_64-unknown-linux-gnu");
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
    }
}
