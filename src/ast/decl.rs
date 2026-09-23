use crate::lexer::SourceSpan;

use super::abi::ForeignAbi;
use super::stmt::Block;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Program {
    pub declarations: Vec<TopLevelDecl>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TopLevelDecl {
    Verb(VerbDecl),
    ExternalVerb(ExternalVerbDecl),
    Struct(StructDef),
    Enum(EnumDef),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumDef {
    pub name: String,
    pub generic_parameters: Vec<GenericParam>,
    pub variants: Vec<EnumVariant>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumVariant {
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
    pub name: String,
    pub ty: TypeName,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructDef {
    pub name: String,
    pub generic_parameters: Vec<GenericParam>,
    pub fields: Vec<StructField>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructField {
    pub role: StructFieldRole,
    pub name: String,
    pub ty: TypeName,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StructFieldRole {
    Value,
    Erg,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalVerbDecl {
    pub unsafe_boundary: bool,
    pub abi: ForeignAbi,
    pub name: String,
    pub generic_parameters: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub return_type: Option<TypeName>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerbDecl {
    pub name: String,
    pub generic_parameters: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub return_type: Option<TypeName>,
    pub body: Block,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Param {
    pub role: Role,
    pub name: String,
    pub ty: TypeName,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Role {
    Erg,
    Abs,
    Dat,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeName {
    pub name: String,
    pub arguments: Vec<TypeName>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenericParam {
    pub name: String,
    pub bound: Option<TypeName>,
    pub span: SourceSpan,
}
