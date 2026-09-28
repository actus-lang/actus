use crate::ast::expr::Expr;
use crate::lexer::SourceSpan;

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
    pub storage: TypeName,
    pub endianness: LayoutEndianness,
    pub fields: Vec<PackField>,
    pub span: SourceSpan,
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
