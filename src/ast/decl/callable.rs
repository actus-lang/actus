use crate::ast::abi::ForeignAbi;
use crate::ast::stmt::Block;
use crate::lexer::SourceSpan;

use super::types::{GenericParam, ReturnType, Role, TypeName};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalVerbDecl {
    pub is_open: bool,
    pub doc: Option<String>,
    pub contract: Option<VerbContract>,
    pub unsafe_boundary: bool,
    pub module_import: bool,
    pub abi: ForeignAbi,
    pub metadata: Vec<MetaAttribute>,
    pub name: String,
    pub generic_parameters: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub return_type: Option<ReturnType>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerbDecl {
    pub is_open: bool,
    pub doc: Option<String>,
    pub contract: Option<VerbContract>,
    pub metadata: Vec<MetaAttribute>,
    pub name: String,
    pub generic_parameters: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub return_type: Option<ReturnType>,
    pub body: Block,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerbContract {
    pub sections: Vec<VerbContractSection>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerbContractSection {
    pub kind: VerbContractSectionKind,
    pub text: String,
    pub span: SourceSpan,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerbContractSectionKind {
    Purpose,
    Inputs,
    Outputs,
    Ownership,
    Invariants,
    Errors,
    SideEffects,
    Abi,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MetaAttribute {
    Test,
    Target(String),
    Limitless(LimitlessScope),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LimitlessScope {
    Verb,
    File,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Param {
    pub role: Role,
    pub name: String,
    pub dispatch: DispatchMode,
    pub ty: TypeName,
    pub span: SourceSpan,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DispatchMode {
    Static,
    Dynamic,
}
