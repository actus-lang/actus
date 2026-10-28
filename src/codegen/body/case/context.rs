use super::super::{BackendBlockId, BackendInstruction, BackendValueId};
use crate::ast::{CaseMode, Expr};
use crate::lexer::SourceSpan;

pub(super) type CaseBranchBlocks = (BackendBlockId, Option<BackendBlockId>, BackendBlockId);

pub(super) struct CaseBranchesInput<'a> {
    pub(super) subject: &'a Expr,
    pub(super) branches: &'a [crate::ast::CaseBranch],
    pub(super) mode: CaseMode,
    pub(super) span: SourceSpan,
    pub(super) fallthrough: Option<BackendBlockId>,
    pub(super) instructions: Vec<BackendInstruction>,
}

pub(super) struct CaseBranchInput<'a> {
    pub(super) subject: &'a Expr,
    pub(super) branch: &'a crate::ast::CaseBranch,
    pub(super) is_last: bool,
    pub(super) blocks: CaseBranchBlocks,
    pub(super) mode: CaseMode,
    pub(super) span: SourceSpan,
    pub(super) fallthrough: Option<BackendBlockId>,
    pub(super) preserve_loop_bindings: bool,
}

pub(super) struct CaseBodyInput<'a> {
    pub(super) id: BackendBlockId,
    pub(super) branch: &'a crate::ast::CaseBranch,
    pub(super) span: SourceSpan,
    pub(super) subject: BackendValueId,
    pub(super) mode: CaseMode,
    pub(super) fallthrough: Option<BackendBlockId>,
    pub(super) guard_failure: BackendBlockId,
}

pub(super) struct BoundCaseInput<'a> {
    pub(super) lowering: &'a mut Vec<BackendInstruction>,
    pub(super) id: BackendBlockId,
    pub(super) branch: &'a crate::ast::CaseBranch,
    pub(super) span: SourceSpan,
    pub(super) fallthrough: Option<BackendBlockId>,
    pub(super) guard_failure: BackendBlockId,
    pub(super) nested_block_start: usize,
    pub(super) scope_bindings: std::collections::BTreeMap<String, BackendValueId>,
}

pub(super) struct GuardedCaseInput<'a> {
    pub(super) id: BackendBlockId,
    pub(super) branch: &'a crate::ast::CaseBranch,
    pub(super) span: SourceSpan,
    pub(super) fallthrough: Option<BackendBlockId>,
    pub(super) guard_failure: BackendBlockId,
    pub(super) guard_instructions: Vec<BackendInstruction>,
    pub(super) nested_block_start: usize,
    pub(super) scope_bindings: std::collections::BTreeMap<String, BackendValueId>,
}
