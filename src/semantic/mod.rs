mod analyzer;
mod borrowing;
mod call_arguments;
mod calls;
mod cleanup;
mod dynamic;
mod enum_constructors;
mod enum_registry;
mod enums;
mod errors;
mod generic_cache;
mod generics;
mod intrinsics;
mod loans;
mod methods;
mod model;
mod origins;
mod ownership;
mod pattern_moves;
mod pattern_support;
mod patterns;
mod roles;
mod scopes;
mod state;
mod structs;
mod type_substitution;
mod types;

pub use analyzer::analyze;
pub use cleanup::{CleanupAction, LoopExitKind, LoopUnwindPlan, ScopeCleanup, UnwindPlan};
pub use errors::{SemanticError, SemanticErrorKind};
pub use model::{
    Binding, BorrowRecord, DynamicRoleType, ExclusiveLoan, FatPointerLayout, GenericInstance,
    Origin, OriginRecord, OriginRoot, ReachablePerformance, SemanticModel,
};
pub use state::{AccessState, OwnershipState, ResourceState};
pub(crate) use type_substitution::TypeSubstitution;
