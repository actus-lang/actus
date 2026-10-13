mod abi;
mod backend;
mod body;
mod body_contract;
mod constants;
mod contract;
#[cfg(feature = "cranelift")]
pub mod cranelift;
mod generic;
mod linker;
#[cfg(feature = "llvm")]
pub mod llvm;
mod model;
mod result_constructors;
mod serialization;
mod shared_lowering;
mod signature;
mod symbols;

pub use abi::{
    BackendAggregateFragment, BackendAggregateLocation, BackendDirectAggregateAbi,
    BackendEnumLayout,
};
pub use backend::{BackendKind, BackendTargetError};
pub use body::{
    BackendBodyProvider, BackendExpressionProvider, lower_verb_body,
    lower_verb_body_with_expressions,
};
pub use contract::{
    BackendAbiError, BackendAggregateField, BackendBlock, BackendBlockId, BackendBlockParameter,
    BackendCaseMode, BackendCleanupAction, BackendCleanupArgument, BackendCleanupBridge,
    BackendCleanupBridgeDescriptor, BackendCleanupField, BackendCleanupOperand,
    BackendCleanupOrigin, BackendCleanupPlan, BackendCleanupProvider,
    BackendCleanupResolvedDescriptor, BackendCleanupResult, BackendContractError,
    BackendEdgeArgument, BackendEmitError, BackendExternal, BackendFailure, BackendFunction,
    BackendInstruction, BackendIntegerBinaryOp, BackendIntegerCompareOp, BackendIntegerUnaryOp,
    BackendLayout, BackendModule, BackendParameter, BackendPassingMode, BackendReturn,
    BackendReturnMode, BackendTargetContext, BackendUnsupportedOperation, BackendValueId,
    BackendValueType, lower_semantic_cleanup, lower_semantic_cleanup_with_provider,
    validate_edge_arguments,
};

/// Aggregates at or above this size use explicit runtime storage instead of a
/// native function stack slot.
pub const LARGE_AGGREGATE_THRESHOLD: u32 = 64 * 1024;
pub use linker::{LinkerPlan, NativeLinkError, link_object, link_objects};
pub use model::{
    LoweredCleanupInstruction, LoweredCleanupPlan, LoweredLoopUnwindPlan, LoweredUnwindPlan,
    lower_cleanup_plans, lower_loop_unwind_plans, lower_return_unwind_plans,
};
#[cfg(test)]
pub(crate) use shared_lowering::lower_program_to_backend_module;
pub(crate) use shared_lowering::lower_program_to_backend_module_for_target;
#[doc(hidden)]
pub use shared_lowering::lower_scalar_program;
pub(crate) use signature::lower_verb_with_expressions_as;
pub use signature::{
    BackendFieldLayout, BackendLayoutProvider, BackendPackedFieldLayout, BackendSignature,
    lower_verb, lower_verb_signature, lower_verb_with_cleanup, lower_verb_with_expressions,
};
pub(crate) use signature::{lower_parameter, lower_return};
pub use symbols::{BackendSymbolBindings, SymbolError, SymbolIdentity, SymbolKind, SymbolRegistry};

#[cfg(feature = "cranelift")]
pub(crate) use cranelift::native::reachable_call_names;
pub(crate) use generic::expand_generic_instances;
pub(crate) use generic::specialize_program;
pub(crate) use generic::specialized_generic_name;
pub(crate) use result_constructors::normalize_program;
