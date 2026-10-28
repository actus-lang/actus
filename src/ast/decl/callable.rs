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
    StagingOpenFailure,
    WriteFailure,
    ShortWrite,
    FlushFailure,
    CloseFailure,
    CleanupFailure,
    RenameFailure,
    InterruptedBeforeRename,
    ParentOpenFailure,
    ParentSyncFailure,
    InterruptedPublication,
}

impl TestFault {
    pub const fn name(self) -> &'static str {
        match self {
            Self::StagingOpenFailure => "staging_open_failure",
            Self::WriteFailure => "write_failure",
            Self::ShortWrite => "short_write",
            Self::FlushFailure => "flush_failure",
            Self::CloseFailure => "close_failure",
            Self::CleanupFailure => "cleanup_failure",
            Self::RenameFailure => "rename_failure",
            Self::InterruptedBeforeRename => "interrupted_before_rename",
            Self::ParentOpenFailure => "parent_open_failure",
            Self::ParentSyncFailure => "parent_sync_failure",
            Self::InterruptedPublication => "interrupted_publication",
        }
    }

    pub const fn code(self) -> i32 {
        match self {
            Self::StagingOpenFailure => 1,
            Self::WriteFailure => 2,
            Self::ShortWrite => 3,
            Self::FlushFailure => 4,
            Self::CloseFailure => 5,
            Self::CleanupFailure => 6,
            Self::RenameFailure => 7,
            Self::InterruptedBeforeRename => 8,
            Self::ParentOpenFailure => 9,
            Self::ParentSyncFailure => 10,
            Self::InterruptedPublication => 11,
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "staging_open_failure" => Some(Self::StagingOpenFailure),
            "write_failure" => Some(Self::WriteFailure),
            "short_write" => Some(Self::ShortWrite),
            "flush_failure" => Some(Self::FlushFailure),
            "close_failure" => Some(Self::CloseFailure),
            "cleanup_failure" => Some(Self::CleanupFailure),
            "rename_failure" => Some(Self::RenameFailure),
            "interrupted_before_rename" => Some(Self::InterruptedBeforeRename),
            "parent_open_failure" => Some(Self::ParentOpenFailure),
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
