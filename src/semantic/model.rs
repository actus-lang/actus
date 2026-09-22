use crate::ast::{BuiltinType, Role};
use crate::lexer::SourceSpan;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BindingState {
    Active,
    Frozen { borrow_ids: Vec<usize> },
    PartiallyMoved { fields: Vec<String> },
    Moved,
    Dropped,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Binding {
    pub name: String,
    pub role: Role,
    pub ty: Option<BuiltinType>,
    pub span: SourceSpan,
    pub state: BindingState,
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
pub struct SemanticModel {
    pub bindings: Vec<Binding>,
    pub borrows: Vec<BorrowRecord>,
    pub cleanup_plans: Vec<super::cleanup::ScopeCleanup>,
    pub return_unwind_plans: Vec<super::cleanup::UnwindPlan>,
    pub loop_unwind_plans: Vec<super::cleanup::LoopUnwindPlan>,
}
