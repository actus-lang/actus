mod abi;
mod backend;
mod bindings;
mod body;
mod constants;
mod contract;
#[cfg(feature = "cranelift")]
pub mod cranelift;
mod enum_layout;
mod linker;
#[cfg(feature = "llvm")]
pub mod llvm;
mod model;
mod result_constructors;
mod serialization;
mod signature;
mod symbols;

pub use abi::{BackendAggregateFragment, BackendAggregateLocation, BackendDirectAggregateAbi};
pub use backend::{BackendKind, BackendTargetError};
pub use bindings::BackendSymbolBindings;
pub use body::{BackendBodyProvider, lower_verb_body};
pub use contract::{
    BackendAbiError, BackendBlock, BackendBlockId, BackendCleanupAction, BackendCleanupField,
    BackendCleanupOperand, BackendCleanupPlan, BackendContractError, BackendEmitError,
    BackendExternal, BackendFailure, BackendFunction, BackendInstruction, BackendLayout,
    BackendModule, BackendParameter, BackendPassingMode, BackendReturn, BackendReturnMode,
    BackendTargetContext, BackendUnsupportedOperation, BackendValueId, BackendValueType,
    lower_semantic_cleanup,
};
pub use enum_layout::BackendEnumLayout;
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

#[cfg(feature = "cranelift")]
pub(crate) use cranelift::generic::expand_generic_instances;
#[cfg(feature = "cranelift")]
pub(crate) use cranelift::generic::specialized_generic_name;
#[cfg(feature = "cranelift")]
pub(crate) use cranelift::native::reachable_call_names;
pub(crate) use result_constructors::normalize_program;
