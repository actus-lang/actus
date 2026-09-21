#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuiltinType {
    Int,
    Buffer,
    Array,
    Map,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BuiltinTypeSpec {
    pub name: &'static str,
}

impl BuiltinType {
    pub const fn spec(self) -> BuiltinTypeSpec {
        match self {
            Self::Int => BuiltinTypeSpec { name: "Int" },
            Self::Buffer => BuiltinTypeSpec { name: "Buffer" },
            Self::Array => BuiltinTypeSpec { name: "Array" },
            Self::Map => BuiltinTypeSpec { name: "Map" },
        }
    }
}

pub fn lookup_builtin_type(name: &str) -> Option<BuiltinType> {
    match name {
        "Int" => Some(BuiltinType::Int),
        "Buffer" => Some(BuiltinType::Buffer),
        "Array" => Some(BuiltinType::Array),
        "Map" => Some(BuiltinType::Map),
        _ => None,
    }
}
