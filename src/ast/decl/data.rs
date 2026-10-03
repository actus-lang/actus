use crate::ast::expr::Expr;
use crate::lexer::SourceSpan;

use super::super::types::{PrimitiveType, primitive_type};
use super::types::{GenericParam, Role, TypeName};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumDef {
    pub is_open: bool,
    pub doc: Option<String>,
    pub name: String,
    pub generic_parameters: Vec<GenericParam>,
    pub variants: Vec<EnumVariant>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumVariant {
    pub doc: Option<String>,
    pub name: String,
    pub payload: EnumPayload,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnumPayload {
    Unit,
    Tuple(Vec<TypeName>),
    Struct(Vec<EnumField>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumField {
    pub doc: Option<String>,
    pub name: String,
    pub ty: TypeName,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructDef {
    pub is_open: bool,
    pub doc: Option<String>,
    pub name: String,
    pub generic_parameters: Vec<GenericParam>,
    pub fields: Vec<StructField>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructField {
    pub doc: Option<String>,
    pub role: StructFieldRole,
    pub name: String,
    pub ty: TypeName,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackDecl {
    pub is_open: bool,
    pub doc: Option<String>,
    pub name: String,
    pub storage_name: String,
    pub storage_doc: Option<String>,
    pub storage: PackStorage,
    pub endianness: LayoutEndianness,
    pub fields: Vec<PackField>,
    pub span: SourceSpan,
}

impl PackDecl {
    pub fn layout_identity(&self) -> String {
        let fields = self
            .fields
            .iter()
            .map(|field| {
                format!(
                    "{}:{}:{}@{}",
                    field_role_name(&field.role),
                    field.name,
                    field.ty.canonical_key(),
                    field.offset
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{}|storage={}|layout={:?}|fields=[{}]",
            self.name,
            self.storage.canonical_key(),
            self.endianness,
            fields
        )
    }
}

/// Frontend-owned representation of a pack's physical storage contract.
///
/// `ByteArray` is accepted by the frontend and semantic layers as an inline,
/// bounded multi-word representation. Native lowering remains a later gate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PackStorage {
    Scalar(TypeName),
    ByteArray { type_name: TypeName, element: TypeName, capacity: u64 },
}

impl PackStorage {
    pub fn type_name(&self) -> &TypeName {
        match self {
            Self::Scalar(type_name) | Self::ByteArray { type_name, .. } => type_name,
        }
    }

    pub fn span(&self) -> SourceSpan {
        self.type_name().span
    }

    pub fn canonical_key(&self) -> String {
        self.type_name().canonical_key()
    }

    pub fn byte_capacity(&self) -> Option<u64> {
        match self {
            Self::ByteArray { capacity, .. } => Some(*capacity),
            Self::Scalar(type_name) => {
                primitive_type(&type_name.name).and_then(|primitive| match primitive {
                    PrimitiveType::Integer { width, .. } if width.is_multiple_of(8) => {
                        Some(u64::from(width / 8))
                    }
                    _ => None,
                })
            }
        }
    }

    pub fn bit_capacity(&self) -> Option<u64> {
        self.byte_capacity()?.checked_mul(8)
    }

    pub fn alignment_bytes(&self) -> Option<u64> {
        match self {
            Self::ByteArray { .. } => Some(1),
            Self::Scalar(type_name) => {
                primitive_type(&type_name.name).and_then(|primitive| match primitive {
                    PrimitiveType::Integer { width, .. } if width.is_multiple_of(8) => {
                        Some(u64::from((width / 8).min(8)))
                    }
                    _ => None,
                })
            }
        }
    }
}

fn field_role_name(role: &Role) -> &'static str {
    match role {
        Role::Erg => "erg",
        Role::Abs => "abs",
        Role::Dat => "dat",
        Role::Ins => "ins",
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackField {
    pub doc: Option<String>,
    pub role: Role,
    pub name: String,
    pub ty: TypeName,
    pub offset: u16,
    pub default_value: Option<Expr>,
    pub span: SourceSpan,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutEndianness {
    Little,
    Big,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StructFieldRole {
    Value,
    Erg,
    Abs,
    Ins,
}
