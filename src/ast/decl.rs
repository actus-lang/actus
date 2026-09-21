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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalVerbDecl {
    pub abi: ForeignAbi,
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeName>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerbDecl {
    pub name: String,
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
    pub span: SourceSpan,
}
