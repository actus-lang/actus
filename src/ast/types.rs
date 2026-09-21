#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuiltinType {
    Int,
    Buffer,
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
        }
    }
}

pub fn lookup_builtin_type(name: &str) -> Option<BuiltinType> {
    match name {
        "Int" => Some(BuiltinType::Int),
        "Buffer" => Some(BuiltinType::Buffer),
        _ => None,
    }
}
