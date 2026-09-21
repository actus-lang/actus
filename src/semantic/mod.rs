mod analyzer;
mod borrowing;
mod calls;
mod cleanup;
mod errors;
mod intrinsics;
mod model;
mod ownership;
mod scopes;

pub use analyzer::analyze;
pub use cleanup::{CleanupAction, LoopExitKind, LoopUnwindPlan, ScopeCleanup, UnwindPlan};
pub use errors::{SemanticError, SemanticErrorKind};
pub use model::{Binding, BindingState, BorrowRecord, SemanticModel};
