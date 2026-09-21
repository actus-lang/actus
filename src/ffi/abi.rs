use crate::ast::{BuiltinType, Role, VerbDecl, lookup_builtin_type};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CAbiType {
    Int32,
    OpaquePointer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CAbiOwnership {
    Exclusive,
    SharedBorrow,
    Consumed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CallingConvention {
    C,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CAbiLayout {
    pub size: u8,
    pub alignment: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CAbiTarget {
    pub pointer_width: u8,
}

impl CAbiTarget {
    pub const fn new(pointer_width: u8) -> Option<Self> {
        match pointer_width {
            4 | 8 => Some(Self { pointer_width }),
            _ => None,
        }
    }

    pub const fn layout(self, ty: CAbiType) -> CAbiLayout {
        match ty {
            CAbiType::Int32 => CAbiLayout { size: 4, alignment: 4 },
            CAbiType::OpaquePointer => {
                CAbiLayout { size: self.pointer_width, alignment: self.pointer_width }
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CAbiParameter {
    pub name: String,
    pub ty: CAbiType,
    pub ownership: CAbiOwnership,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CAbiSignature {
    pub name: String,
    pub parameters: Vec<CAbiParameter>,
    pub return_type: CAbiType,
    pub calling_convention: CallingConvention,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CAbiError {
    MissingReturnType { verb: String },
    UnsupportedType { name: String },
}

impl std::fmt::Display for CAbiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingReturnType { verb } => {
                write!(formatter, "C ABI export `{verb}` requires an explicit return type")
            }
            Self::UnsupportedType { name } => {
                write!(formatter, "type `{name}` has no C ABI mapping")
            }
        }
    }
}

impl std::error::Error for CAbiError {}

pub fn c_abi_signature(verb: &VerbDecl) -> Result<CAbiSignature, CAbiError> {
    let return_type = verb
        .return_type
        .as_ref()
        .ok_or_else(|| CAbiError::MissingReturnType { verb: verb.name.clone() })?;
    let parameters = verb
        .params
        .iter()
        .map(|parameter| {
            Ok(CAbiParameter {
                name: parameter.name.clone(),
                ty: map_type(&parameter.ty.name)?,
                ownership: map_ownership(&parameter.role),
            })
        })
        .collect::<Result<Vec<_>, CAbiError>>()?;
    Ok(CAbiSignature {
        name: verb.name.clone(),
        parameters,
        return_type: map_type(&return_type.name)?,
        calling_convention: CallingConvention::C,
    })
}

fn map_type(name: &str) -> Result<CAbiType, CAbiError> {
    match lookup_builtin_type(name) {
        Some(BuiltinType::Int) => Ok(CAbiType::Int32),
        Some(BuiltinType::Buffer) => Ok(CAbiType::OpaquePointer),
        Some(BuiltinType::Array | BuiltinType::Map) | None => {
            Err(CAbiError::UnsupportedType { name: name.to_owned() })
        }
    }
}

fn map_ownership(role: &Role) -> CAbiOwnership {
    match role {
        Role::Erg => CAbiOwnership::Exclusive,
        Role::Abs => CAbiOwnership::SharedBorrow,
        Role::Dat => CAbiOwnership::Consumed,
    }
}
