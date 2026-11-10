mod analyzer;
mod array_capacity;
mod borrowing;
mod calls;
mod cleanup;
mod dynamic;
mod enums;
mod errors;
mod generated_locals;
mod generics;
mod intrinsics;
mod loans;
mod methods;
mod model;
mod origins;
mod ownership;
mod packs;
mod patterns;
mod print_validation;
mod repr;
mod roles;
mod scopes;
mod serialization;
mod stack_limit;
mod state;
mod structs;
mod target_filter;
mod types;
mod volatile;

pub use analyzer::analyze;
pub(crate) use analyzer::validate_bounded_recursion;
pub(crate) use analyzer::{
    validate_stack_limit_dynamic_dispatch, validate_stack_limit_native_claims,
    validate_stack_limit_recursion,
};
pub(crate) use calls::parse_type_name_key;
pub use cleanup::{CleanupAction, LoopExitKind, LoopUnwindPlan, ScopeCleanup, UnwindPlan};
pub use errors::{SemanticError, SemanticErrorKind};
pub(crate) use generated_locals::normalize_program as normalize_generated_locals;
pub(crate) use generics::substitution::TypeSubstitution;
pub use model::{
    ArgumentRoleFact, ArgumentRoleSource, Binding, BorrowRecord, ConditionalFact, DynamicRoleType,
    ExclusiveLoan, ExplicitDrop, FatPointerLayout, GenericInstance, LiteralFact, Origin,
    OriginRecord, OriginRoot, PackFieldContract, PackLayoutContract, ReachablePerformance,
    SemanticModel, SerializationContract,
};
pub use stack_limit::validate_stack_limit_backend_target;
pub use state::{AccessState, OwnershipState, ResourceState};
pub use target_filter::{filter_program_for_target, filter_program_for_target_checked};
pub use types::{SemanticType, TypeRegistry};
