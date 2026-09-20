use crate::ast::Role;
use crate::lexer::SourceSpan;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BindingState {
    Active,
    Frozen { borrow_ids: Vec<usize> },
    Moved,
    Dropped,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Binding {
    pub name: String,
    pub role: Role,
    pub span: SourceSpan,
    pub state: BindingState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BorrowRecord {
    pub id: usize,
    pub owner: String,
    pub scope_depth: usize,
    pub origin_span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticModel {
    pub bindings: Vec<Binding>,
    pub borrows: Vec<BorrowRecord>,
}
