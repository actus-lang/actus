use crate::ast::{BuiltinType, Role, TypeName};
use crate::lexer::SourceSpan;

use super::state::{AccessState, OwnershipState};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Binding {
    pub name: String,
    pub role: Role,
    pub ty: Option<BuiltinType>,
    pub span: SourceSpan,
    pub ownership: OwnershipState,
    pub access: AccessState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BorrowRecord {
    pub id: usize,
    pub owner: String,
    pub field: Option<String>,
    pub scope_depth: usize,
    pub origin_span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExclusiveLoan {
    pub id: usize,
    pub owner: String,
    pub callee: String,
    pub parameter: String,
    pub origin_span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Origin {
    None,
    AbsParameter { parameter_index: usize },
    Derived { root_parameter: usize },
    Binding { binding_index: usize },
    Unknown,
    Multiple { roots: Vec<OriginRoot> },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum OriginRoot {
    Parameter(usize),
    Binding(usize),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OriginRecord {
    pub span: SourceSpan,
    pub origin: Origin,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenericInstance {
    pub name: String,
    pub arguments: Vec<TypeName>,
    pub canonical_key: String,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ReachablePerformance {
    pub role_name: String,
    pub target_type: String,
    pub method_name: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FatPointerLayout {
    pub data_ptr_word: u8,
    pub vtable_ptr_word: u8,
    pub word_count: u8,
    pub alignment_words: u8,
}

impl FatPointerLayout {
    pub const DYNAMIC_ROLE: Self =
        Self { data_ptr_word: 0, vtable_ptr_word: 1, word_count: 2, alignment_words: 1 };
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DynamicRoleType {
    pub role_name: String,
    pub layout: FatPointerLayout,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticModel {
    pub bindings: Vec<Binding>,
    pub borrows: Vec<BorrowRecord>,
    pub exclusive_loans: Vec<ExclusiveLoan>,
    pub expression_origins: Vec<OriginRecord>,
    pub cleanup_plans: Vec<super::cleanup::ScopeCleanup>,
    pub return_unwind_plans: Vec<super::cleanup::UnwindPlan>,
    pub loop_unwind_plans: Vec<super::cleanup::LoopUnwindPlan>,
    pub generic_instances: Vec<GenericInstance>,
    pub reachable_performances: Vec<ReachablePerformance>,
    pub dynamic_roles: Vec<DynamicRoleType>,
}
