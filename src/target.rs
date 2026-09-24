use std::fmt::{Display, Formatter};
use std::str::FromStr;

use target_lexicon::{
    Architecture, BinaryFormat, CallingConvention, Endianness, PointerWidth, Triple,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TargetSpec {
    triple: Triple,
    pub architecture: Architecture,
    pub pointer_width: PointerWidth,
    pub endianness: Endianness,
    pub object_format: BinaryFormat,
    pub abi: CallingConvention,
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
        Ok(Self {
            architecture: triple.architecture,
            pointer_width,
            endianness,
            object_format: triple.binary_format,
            abi,
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

    pub fn spec_hash(&self) -> String {
        let identity = format!(
            "triple={};architecture={:?};pointer_width={:?};endianness={:?};object_format={:?};abi={:?}",
            self.triple,
            self.architecture,
            self.pointer_width,
            self.endianness,
            self.object_format,
            self.abi
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

    use super::TargetSpec;

    #[test]
    fn derives_target_contract_fields_from_a_triple() {
        let spec = TargetSpec::parse("x86_64-unknown-linux-gnu").expect("target should parse");
        assert_eq!(spec.pointer_width, PointerWidth::U64);
        assert_eq!(spec.endianness, Endianness::Little);
        assert_eq!(spec.object_format.into_str(), "elf");
        assert_eq!(spec.abi, CallingConvention::SystemV);
        assert_eq!(spec.triple().to_string(), "x86_64-unknown-linux-gnu");
    }
}
