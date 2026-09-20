mod analyzer;
mod borrowing;
mod calls;
mod cleanup;
mod errors;
mod model;
mod ownership;
mod scopes;

pub use analyzer::analyze;
pub use cleanup::{CleanupAction, ScopeCleanup, UnwindPlan};
pub use errors::{SemanticError, SemanticErrorKind};
pub use model::{Binding, BindingState, BorrowRecord, SemanticModel};
