use crate::lexer::SourceSpan;

use super::callable::{ExternalVerbDecl, Param, VerbDecl};
use super::data::{EnumDef, PackDecl, StructDef};
use super::types::{ReturnType, TypeName};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Program {
    pub declarations: Vec<TopLevelDecl>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TopLevelDecl {
    Verb(VerbDecl),
    ExternalVerb(ExternalVerbDecl),
    Struct(StructDef),
    Pack(PackDecl),
    Enum(EnumDef),
    Role(RoleDecl),
    Perform(PerformDecl),
    OpenSibling(OpenSiblingDecl),
    Import(ImportDecl),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenSiblingDecl {
    pub doc: Option<String>,
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportDecl {
    pub path: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleDecl {
    pub is_open: bool,
    pub doc: Option<String>,
    pub name: String,
    pub methods: Vec<RoleMethod>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleMethod {
    pub doc: Option<String>,
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<ReturnType>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PerformDecl {
    pub is_open: bool,
    pub doc: Option<String>,
    pub role_name: String,
    pub target: TypeName,
    pub methods: Vec<VerbDecl>,
    pub span: SourceSpan,
}
