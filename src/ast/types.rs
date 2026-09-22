#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuiltinType {
    Int,
    Bool,
    String,
    Buffer,
    Array,
    Map,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BuiltinTypeSpec {
    pub name: &'static str,
    pub status: RegistryStatus,
}

impl BuiltinType {
    pub const fn spec(self) -> BuiltinTypeSpec {
        match self {
            Self::Int => BuiltinTypeSpec { name: "Int", status: RegistryStatus::Active },
            Self::Bool => BuiltinTypeSpec { name: "Bool", status: RegistryStatus::Active },
            Self::String => BuiltinTypeSpec { name: "String", status: RegistryStatus::Active },
            Self::Buffer => BuiltinTypeSpec { name: "Buffer", status: RegistryStatus::Active },
            Self::Array => BuiltinTypeSpec { name: "Array", status: RegistryStatus::Active },
            Self::Map => BuiltinTypeSpec { name: "Map", status: RegistryStatus::Active },
        }
    }
}

pub fn lookup_builtin_type(name: &str) -> Option<BuiltinType> {
    match name {
        "Int" => Some(BuiltinType::Int),
        "Bool" => Some(BuiltinType::Bool),
        "String" => Some(BuiltinType::String),
        "Buffer" => Some(BuiltinType::Buffer),
        "Array" => Some(BuiltinType::Array),
        "Map" => Some(BuiltinType::Map),
        _ => None,
    }
}
use super::RegistryStatus;
