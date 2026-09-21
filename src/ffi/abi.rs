use crate::ast::{BuiltinType, ExternalVerbDecl, Role, VerbDecl, lookup_builtin_type};

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
pub enum CAbiReturnOwnership {
    Value,
    OwnedResource,
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
    pub return_ownership: CAbiReturnOwnership,
    pub calling_convention: CallingConvention,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CAbiError {
    MissingReturnType { verb: String },
    UnsupportedType { name: String },
    UnsupportedCallingConvention { abi: String },
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
            Self::UnsupportedCallingConvention { abi } => {
                write!(formatter, "calling convention `{abi}` is not supported")
            }
        }
    }
}

impl std::error::Error for CAbiError {}

pub fn c_abi_signature(verb: &VerbDecl) -> Result<CAbiSignature, CAbiError> {
    signature_parts(&verb.name, &verb.params, verb.return_type.as_ref())
}

pub fn c_abi_external_signature(
    declaration: &ExternalVerbDecl,
) -> Result<CAbiSignature, CAbiError> {
    if declaration.abi != "C" {
        return Err(CAbiError::UnsupportedCallingConvention { abi: declaration.abi.clone() });
    }
    signature_parts(&declaration.name, &declaration.params, declaration.return_type.as_ref())
}

fn signature_parts(
    name: &str,
    params: &[crate::ast::Param],
    return_type: Option<&crate::ast::TypeName>,
) -> Result<CAbiSignature, CAbiError> {
    let return_type =
        return_type.ok_or_else(|| CAbiError::MissingReturnType { verb: name.to_owned() })?;
    let parameters = params
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
        name: name.to_owned(),
        parameters,
        return_type: map_type(&return_type.name)?,
        return_ownership: return_ownership(&return_type.name)?,
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

fn return_ownership(name: &str) -> Result<CAbiReturnOwnership, CAbiError> {
    match map_type(name)? {
        CAbiType::Int32 => Ok(CAbiReturnOwnership::Value),
        CAbiType::OpaquePointer => Ok(CAbiReturnOwnership::OwnedResource),
    }
}
