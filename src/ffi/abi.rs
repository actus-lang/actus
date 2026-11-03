use crate::ast::{
    BuiltinType, ExternalVerbDecl, ForeignAbi, Program, ReturnAccess, ReturnType, Role, VerbDecl,
    lookup_builtin_type,
};

#[path = "abi_errors.rs"]
mod abi_errors;
#[path = "aggregate.rs"]
mod aggregate;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CAbiType {
    Int32,
    Integer { signed: bool, width: u8 },
    Float { width: u8 },
    Bool,
    Usize,
    OpaquePointer,
    Array { element: Box<CAbiType>, length: u32, layout: CAbiLayout },
    Aggregate(CAbiAggregate),
    Enum { name: String, layout: CAbiLayout },
    TaggedUnion(CAbiTaggedUnion),
}

impl CAbiType {
    pub fn c_name(&self) -> String {
        match self {
            Self::Int32 => "int32_t".to_owned(),
            Self::Integer { signed, width } => {
                format!("{}int{width}_t", if *signed { "" } else { "u" })
            }
            Self::Float { width: 32 } => "float".to_owned(),
            Self::Float { width: 64 } => "double".to_owned(),
            Self::Float { width } => format!("float{width}_t"),
            Self::Bool => "uint8_t".to_owned(),
            Self::Usize => "uintptr_t".to_owned(),
            Self::OpaquePointer => "void*".to_owned(),
            Self::Array { element, length, .. } => {
                format!("{}[{length}]", element.c_name())
            }
            Self::Aggregate(aggregate) => format!("struct {}", aggregate.name),
            Self::Enum { name, .. } => format!("enum {name}"),
            Self::TaggedUnion(union) => format!("struct {}", union.name),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CAbiAggregate {
    pub name: String,
    pub packed: bool,
    pub layout: CAbiLayout,
    pub fields: Vec<CAbiAggregateField>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CAbiAggregateField {
    pub name: String,
    pub offset: u32,
    pub layout: CAbiLayout,
    pub ty: CAbiType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CAbiTaggedUnion {
    pub name: String,
    pub tag: CAbiLayout,
    pub payload_offset: u32,
    pub payload: Option<CAbiLayout>,
    pub layout: CAbiLayout,
    pub variants: Vec<CAbiTaggedUnionVariant>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CAbiTaggedUnionVariant {
    pub name: String,
    pub discriminant: u32,
    pub payload: Option<CAbiAggregate>,
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
    Borrowed,
    OwnedResource,
    CallerAllocated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CAbiParameterPassing {
    Direct,
    Pointer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CallingConvention {
    C,
}

impl CallingConvention {
    pub const fn name(self) -> &'static str {
        match self {
            Self::C => "C",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CAbiLayout {
    pub size: u32,
    pub alignment: u32,
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

    pub fn layout(self, ty: CAbiType) -> CAbiLayout {
        match ty {
            CAbiType::Int32 => CAbiLayout { size: 4, alignment: 4 },
            CAbiType::Integer { width, .. } if width.is_multiple_of(8) => {
                let size = u32::from(width / 8);
                CAbiLayout { size, alignment: size.min(u32::from(self.pointer_width)) }
            }
            CAbiType::Float { width } if width.is_multiple_of(8) => {
                let size = u32::from(width / 8);
                CAbiLayout { size, alignment: size.min(u32::from(self.pointer_width)) }
            }
            CAbiType::Bool => CAbiLayout { size: 1, alignment: 1 },
            CAbiType::Usize | CAbiType::OpaquePointer => CAbiLayout {
                size: u32::from(self.pointer_width),
                alignment: u32::from(self.pointer_width),
            },
            CAbiType::Array { layout, .. }
            | CAbiType::Aggregate(CAbiAggregate { layout, .. })
            | CAbiType::Enum { layout, .. } => layout,
            CAbiType::TaggedUnion(union) => union.layout,
            CAbiType::Integer { .. } | CAbiType::Float { .. } => {
                unreachable!("non-byte-aligned C ABI scalar")
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CAbiParameter {
    pub name: String,
    pub ty: CAbiType,
    pub ownership: CAbiOwnership,
    pub passing: CAbiParameterPassing,
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
    MissingUnsafeBoundary { verb: String },
    InvalidOwnership { parameter: String, role: String, ty: String },
    InvalidReturnOwnership { access: String, ty: String },
    UnsupportedType { name: String },
    AggregateRequiresReprC { name: String },
    TransparentCombinedRepresentation { name: String },
    TransparentRequiresOneField { name: String },
    TransparentFieldRole { aggregate: String, field: String, role: String },
    TransparentFieldType { aggregate: String, field: String, ty: String },
    AggregateFieldRole { aggregate: String, field: String, role: String },
    AggregateFieldType { aggregate: String, field: String, ty: String },
    AggregateLayoutOverflow { name: String },
    GenericAggregate { name: String },
    TargetDependentType { name: String },
}

pub fn c_abi_signature(verb: &VerbDecl) -> Result<CAbiSignature, CAbiError> {
    signature_parts(&verb.name, &verb.params, verb.return_type.as_ref())
}

pub fn c_abi_external_signature(
    declaration: &ExternalVerbDecl,
) -> Result<CAbiSignature, CAbiError> {
    if !declaration.unsafe_boundary && !declaration.module_import {
        return Err(CAbiError::MissingUnsafeBoundary { verb: declaration.name.clone() });
    }
    match declaration.abi {
        ForeignAbi::C => {}
    }
    signature_parts(&declaration.name, &declaration.params, declaration.return_type.as_ref())
}

pub fn c_abi_signature_in_program(
    program: &Program,
    verb: &VerbDecl,
) -> Result<CAbiSignature, CAbiError> {
    signature_parts_with_program(
        CAbiTarget::new(8).expect("64-bit target"),
        program,
        &verb.name,
        &verb.params,
        verb.return_type.as_ref(),
    )
}

pub fn c_abi_signature_in_program_for_target(
    target: CAbiTarget,
    program: &Program,
    verb: &VerbDecl,
) -> Result<CAbiSignature, CAbiError> {
    signature_parts_with_program(
        target,
        program,
        &verb.name,
        &verb.params,
        verb.return_type.as_ref(),
    )
}

pub fn c_abi_external_signature_in_program(
    program: &Program,
    declaration: &ExternalVerbDecl,
) -> Result<CAbiSignature, CAbiError> {
    c_abi_external_signature_in_program_for_target(
        CAbiTarget::new(8).expect("64-bit target"),
        program,
        declaration,
    )
}

pub fn c_abi_external_signature_in_program_for_target(
    target: CAbiTarget,
    program: &Program,
    declaration: &ExternalVerbDecl,
) -> Result<CAbiSignature, CAbiError> {
    if !declaration.unsafe_boundary && !declaration.module_import {
        return Err(CAbiError::MissingUnsafeBoundary { verb: declaration.name.clone() });
    }
    signature_parts_with_program(
        target,
        program,
        &declaration.name,
        &declaration.params,
        declaration.return_type.as_ref(),
    )
}

fn signature_parts(
    name: &str,
    params: &[crate::ast::Param],
    return_type: Option<&ReturnType>,
) -> Result<CAbiSignature, CAbiError> {
    let return_type =
        return_type.ok_or_else(|| CAbiError::MissingReturnType { verb: name.to_owned() })?;
    let parameters = params
        .iter()
        .map(|parameter| {
            let ty = map_type(&parameter.ty.name)?;
            validate_parameter_ownership(parameter, &ty)?;
            Ok(CAbiParameter {
                name: parameter.name.clone(),
                passing: passing_for(&ty),
                ty,
                ownership: map_ownership(&parameter.role),
            })
        })
        .collect::<Result<Vec<_>, CAbiError>>()?;
    Ok(CAbiSignature {
        name: name.to_owned(),
        parameters,
        return_type: map_type(&return_type.ty.name)?,
        return_ownership: return_ownership(return_type)?,
        calling_convention: CallingConvention::C,
    })
}

fn signature_parts_with_program(
    target: CAbiTarget,
    program: &Program,
    name: &str,
    params: &[crate::ast::Param],
    return_type: Option<&ReturnType>,
) -> Result<CAbiSignature, CAbiError> {
    let return_type =
        return_type.ok_or_else(|| CAbiError::MissingReturnType { verb: name.to_owned() })?;
    let parameters = params
        .iter()
        .map(|parameter| {
            let ty = aggregate::map_type_in_program(target, program, &parameter.ty)?;
            validate_parameter_ownership(parameter, &ty)?;
            Ok(CAbiParameter {
                name: parameter.name.clone(),
                passing: passing_for(&ty),
                ty,
                ownership: map_ownership(&parameter.role),
            })
        })
        .collect::<Result<Vec<_>, CAbiError>>()?;
    let mapped_return = aggregate::map_type_in_program(target, program, &return_type.ty)?;
    Ok(CAbiSignature {
        name: name.to_owned(),
        parameters,
        return_ownership: return_ownership_for(&mapped_return, return_type)?,
        return_type: mapped_return,
        calling_convention: CallingConvention::C,
    })
}

fn map_type(name: &str) -> Result<CAbiType, CAbiError> {
    match lookup_builtin_type(name) {
        Some(BuiltinType::Int) => Ok(CAbiType::Int32),
        Some(
            BuiltinType::Buffer
            | BuiltinType::String
            | BuiltinType::VolatileAddress
            | BuiltinType::VolatileWriteAddress,
        ) => Ok(CAbiType::OpaquePointer),
        Some(BuiltinType::Bool | BuiltinType::Array | BuiltinType::Map) | None => {
            Err(CAbiError::UnsupportedType { name: name.to_owned() })
        }
    }
}

fn validate_parameter_ownership(
    parameter: &crate::ast::Param,
    ty: &CAbiType,
) -> Result<(), CAbiError> {
    if matches!(parameter.role, Role::Abs | Role::Dat)
        && !matches!(
            ty,
            CAbiType::OpaquePointer | CAbiType::Aggregate(_) | CAbiType::TaggedUnion(_)
        )
    {
        return Err(CAbiError::InvalidOwnership {
            parameter: parameter.name.clone(),
            role: role_name(&parameter.role).to_owned(),
            ty: parameter.ty.name.clone(),
        });
    }
    Ok(())
}

fn passing_for(ty: &CAbiType) -> CAbiParameterPassing {
    match ty {
        CAbiType::Int32
        | CAbiType::Integer { .. }
        | CAbiType::Float { .. }
        | CAbiType::Bool
        | CAbiType::Usize
        | CAbiType::Enum { .. } => CAbiParameterPassing::Direct,
        CAbiType::OpaquePointer
        | CAbiType::Array { .. }
        | CAbiType::Aggregate(_)
        | CAbiType::TaggedUnion(_) => CAbiParameterPassing::Pointer,
    }
}

fn role_name(role: &Role) -> &'static str {
    match role {
        Role::Erg => "erg",
        Role::Abs => "abs",
        Role::Dat => "dat",
        Role::Ins => "ins",
    }
}

fn map_ownership(role: &Role) -> CAbiOwnership {
    match role {
        Role::Erg => CAbiOwnership::Exclusive,
        Role::Abs => CAbiOwnership::SharedBorrow,
        Role::Dat => CAbiOwnership::Consumed,
        Role::Ins => CAbiOwnership::Exclusive,
    }
}

fn return_ownership(return_type: &ReturnType) -> Result<CAbiReturnOwnership, CAbiError> {
    let mapped = map_type(&return_type.ty.name)?;
    return_ownership_for(&mapped, return_type)
}

fn return_ownership_for(
    mapped: &CAbiType,
    return_type: &ReturnType,
) -> Result<CAbiReturnOwnership, CAbiError> {
    if return_type.access == ReturnAccess::Abs && *mapped != CAbiType::OpaquePointer {
        if matches!(mapped, CAbiType::Aggregate(_)) {
            return Ok(CAbiReturnOwnership::Borrowed);
        }
        return Err(CAbiError::InvalidReturnOwnership {
            access: "abs".to_owned(),
            ty: return_type.ty.name.clone(),
        });
    }
    match (return_type.access, mapped) {
        (ReturnAccess::Owned, CAbiType::OpaquePointer) => Ok(CAbiReturnOwnership::OwnedResource),
        (ReturnAccess::Abs, CAbiType::OpaquePointer)
        | (ReturnAccess::Abs, CAbiType::Aggregate(_))
        | (ReturnAccess::Abs, CAbiType::TaggedUnion(_)) => Ok(CAbiReturnOwnership::Borrowed),
        (ReturnAccess::Owned, CAbiType::Aggregate(_))
        | (ReturnAccess::Owned, CAbiType::TaggedUnion(_)) => {
            Ok(CAbiReturnOwnership::CallerAllocated)
        }
        (ReturnAccess::Owned, _) => Ok(CAbiReturnOwnership::Value),
        (ReturnAccess::Abs, _) => unreachable!("scalar abs returns are rejected above"),
    }
}
