mod analyzer;
mod borrowing;
mod calls;
mod errors;
mod model;
mod ownership;
mod scopes;

pub use analyzer::analyze;
pub use errors::{SemanticError, SemanticErrorKind};
pub use model::{Binding, BindingState, BorrowRecord, SemanticModel};
