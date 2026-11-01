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
    MustUse(String),
    Effect(Effect),
    Deterministic,
    Realtime,
    Bounded(BoundedContract),
    Capability(Capability),
    Repr(Repr),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Repr {
    c: bool,
    packed: bool,
}

impl Repr {
    pub const C: Self = Self { c: true, packed: false };
    pub const PACKED: Self = Self { c: false, packed: true };

    pub const fn new(c: bool, packed: bool) -> Self {
        Self { c, packed }
    }

    pub const fn is_c(self) -> bool {
        self.c
    }

    pub const fn is_packed(self) -> bool {
        self.packed
    }

    pub fn name(self) -> String {
        match (self.c, self.packed) {
            (true, true) => "C, packed".to_owned(),
            (true, false) => "C".to_owned(),
            (false, true) => "packed".to_owned(),
            (false, false) => String::new(),
        }
    }

    pub const fn combine(self, other: Self) -> Self {
        Self { c: self.c || other.c, packed: self.packed || other.packed }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundedContract {
    pub max_iterations: Option<u32>,
    pub max_call_depth: Option<u32>,
    pub max_calls: Option<u32>,
    pub max_allocations: Option<u32>,
    pub max_allocation_bytes: Option<u64>,
    pub max_stack_bytes: Option<u64>,
    pub max_io_bytes: Option<u64>,
}

impl BoundedContract {
    pub fn is_empty(&self) -> bool {
        self.max_iterations.is_none()
            && self.max_call_depth.is_none()
            && self.max_calls.is_none()
            && self.max_allocations.is_none()
            && self.max_allocation_bytes.is_none()
            && self.max_stack_bytes.is_none()
            && self.max_io_bytes.is_none()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Effect {
    Blocking,
    Clock,
    Filesystem,
    Network,
    Allocation,
    Entropy,
}

impl Effect {
    pub const ALL: [Self; 6] = [
        Self::Blocking,
        Self::Clock,
        Self::Filesystem,
        Self::Network,
        Self::Allocation,
        Self::Entropy,
    ];

    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "blocking" => Self::Blocking,
            "clock" => Self::Clock,
            "filesystem" => Self::Filesystem,
            "network" => Self::Network,
            "allocation" => Self::Allocation,
            "entropy" => Self::Entropy,
            _ => return None,
        })
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Blocking => "blocking",
            Self::Clock => "clock",
            Self::Filesystem => "filesystem",
            Self::Network => "network",
            Self::Allocation => "allocation",
            Self::Entropy => "entropy",
        }
    }

    pub const fn is_nondeterministic(self) -> bool {
        matches!(self, Self::Clock | Self::Filesystem | Self::Network | Self::Entropy)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Capability {
    FilesystemCreateExclusive,
    PosixCloexec,
    PosixCreateExclusive,
    UnixRenameAtomic,
    WindowsFileIdentity,
    WindowsCreateNew,
    WindowsReplaceFile,
}

impl Capability {
    pub const ALL: [Self; 7] = [
        Self::FilesystemCreateExclusive,
        Self::PosixCloexec,
        Self::PosixCreateExclusive,
        Self::UnixRenameAtomic,
        Self::WindowsFileIdentity,
        Self::WindowsCreateNew,
        Self::WindowsReplaceFile,
    ];

    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "filesystem.create_exclusive" => Self::FilesystemCreateExclusive,
            "posix.cloexec" => Self::PosixCloexec,
            "posix.create_exclusive" => Self::PosixCreateExclusive,
            "unix.rename_atomic" => Self::UnixRenameAtomic,
            "windows.file_identity" => Self::WindowsFileIdentity,
            "windows.create_new" => Self::WindowsCreateNew,
            "windows.replace_file" => Self::WindowsReplaceFile,
            _ => return None,
        })
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::FilesystemCreateExclusive => "filesystem.create_exclusive",
            Self::PosixCloexec => "posix.cloexec",
            Self::PosixCreateExclusive => "posix.create_exclusive",
            Self::UnixRenameAtomic => "unix.rename_atomic",
            Self::WindowsFileIdentity => "windows.file_identity",
            Self::WindowsCreateNew => "windows.create_new",
            Self::WindowsReplaceFile => "windows.replace_file",
        }
    }
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
    pub const ALL: [Self; 11] = [
        Self::StagingOpenFailure,
        Self::WriteFailure,
        Self::ShortWrite,
        Self::FlushFailure,
        Self::CloseFailure,
        Self::CleanupFailure,
        Self::RenameFailure,
        Self::InterruptedBeforeRename,
        Self::ParentOpenFailure,
        Self::ParentSyncFailure,
        Self::InterruptedPublication,
    ];

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
