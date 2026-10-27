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
    /// Native ABI symbol used when generic specialization changes the Actus name.
    pub native_symbol: Option<String>,
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
    Fault(TestFault),
    Limitless(LimitlessScope),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TestFault {
    ShortWrite,
    FlushFailure,
    RenameFailure,
    ParentSyncFailure,
    InterruptedPublication,
}

impl TestFault {
    pub const fn name(self) -> &'static str {
        match self {
            Self::ShortWrite => "short_write",
            Self::FlushFailure => "flush_failure",
            Self::RenameFailure => "rename_failure",
            Self::ParentSyncFailure => "parent_sync_failure",
            Self::InterruptedPublication => "interrupted_publication",
        }
    }

    pub const fn code(self) -> i32 {
        match self {
            Self::ShortWrite => 1,
            Self::FlushFailure => 2,
            Self::RenameFailure => 3,
            Self::ParentSyncFailure => 4,
            Self::InterruptedPublication => 5,
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "short_write" => Some(Self::ShortWrite),
            "flush_failure" => Some(Self::FlushFailure),
            "rename_failure" => Some(Self::RenameFailure),
            "parent_sync_failure" => Some(Self::ParentSyncFailure),
            "interrupted_publication" => Some(Self::InterruptedPublication),
            _ => None,
        }
    }
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
