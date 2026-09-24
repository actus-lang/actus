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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticModel {
    pub bindings: Vec<Binding>,
    pub borrows: Vec<BorrowRecord>,
    pub cleanup_plans: Vec<super::cleanup::ScopeCleanup>,
    pub return_unwind_plans: Vec<super::cleanup::UnwindPlan>,
    pub loop_unwind_plans: Vec<super::cleanup::LoopUnwindPlan>,
    pub generic_instances: Vec<GenericInstance>,
    pub reachable_performances: Vec<ReachablePerformance>,
}
