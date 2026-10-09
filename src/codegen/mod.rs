mod backend;
mod bindings;
mod body;
mod constants;
mod contract;
pub mod cranelift;
mod linker;
mod model;
mod result_constructor_spans;
mod result_constructors;
mod serialization_generation;
mod serialization_migration;
mod signature;
mod symbols;

pub use backend::{BackendKind, BackendTargetError};
pub use bindings::BackendSymbolBindings;
pub use body::{BackendBodyProvider, lower_verb_body};
pub use contract::{
    BackendAbiError, BackendBlock, BackendBlockId, BackendCleanupAction, BackendCleanupPlan,
    BackendContractError, BackendEmitError, BackendExternal, BackendFailure, BackendFunction,
    BackendInstruction, BackendLayout, BackendModule, BackendParameter, BackendPassingMode,
    BackendReturn, BackendReturnMode, BackendTargetContext, BackendUnsupportedOperation,
    BackendValueId, BackendValueType, lower_semantic_cleanup,
};
pub use linker::{LinkerPlan, NativeLinkError, link_object, link_objects};
pub use model::{
    LoweredCleanupInstruction, LoweredCleanupPlan, LoweredLoopUnwindPlan, LoweredUnwindPlan,
    lower_cleanup_plans, lower_loop_unwind_plans, lower_return_unwind_plans,
};
pub use signature::{
    BackendLayoutProvider, BackendSignature, lower_verb, lower_verb_signature,
    lower_verb_with_cleanup,
};
pub use symbols::{SymbolError, SymbolIdentity, SymbolKind, SymbolRegistry};

pub(crate) use cranelift::generic::expand_generic_instances;
pub(crate) use cranelift::generic::specialized_generic_name;
pub(crate) use cranelift::native::reachable_call_names;
pub(crate) use result_constructors::normalize_program;
