mod analyzer;
mod model;

pub use analyzer::{SemanticError, SemanticErrorKind, analyze};
pub use model::{Binding, BindingState, BorrowRecord, SemanticModel};
