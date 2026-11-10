use std::fmt::{Display, Formatter};

use target_lexicon::{
    Architecture, BinaryFormat, CallingConvention, Endianness, PointerWidth, Triple,
};

use super::contracts::LinkerFlavor;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TargetSpec {
    pub(super) triple: Triple,
    pub architecture: Architecture,
    pub pointer_width: PointerWidth,
    pub endianness: Endianness,
    pub object_format: BinaryFormat,
    pub abi: CallingConvention,
    pub(super) linker_flavor: LinkerFlavor,
}

#[derive(Debug, Eq, PartialEq)]
pub struct TargetSpecError(pub(super) String);

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
