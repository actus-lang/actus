mod abi;
mod backend;
mod body;
mod body_contract;
mod constants;
mod contract;
#[cfg(feature = "cranelift")]
pub mod cranelift;
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
    BackendAbiError, BackendBlock, BackendBlockId, BackendCaseMode, BackendCleanupAction,
    BackendCleanupArgument, BackendCleanupBridge, BackendCleanupBridgeDescriptor,
    BackendCleanupField, BackendCleanupOperand, BackendCleanupOrigin, BackendCleanupPlan,
    BackendCleanupProvider, BackendCleanupResolvedDescriptor, BackendCleanupResult,
    BackendContractError, BackendEmitError, BackendExternal, BackendFailure, BackendFunction,
    BackendInstruction, BackendIntegerBinaryOp, BackendIntegerCompareOp, BackendIntegerUnaryOp,
    BackendLayout, BackendModule, BackendParameter, BackendPassingMode, BackendReturn,
    BackendReturnMode, BackendTargetContext, BackendUnsupportedOperation, BackendValueId,
    BackendValueType, lower_semantic_cleanup, lower_semantic_cleanup_with_provider,
};
pub use linker::{LinkerPlan, NativeLinkError, link_object, link_objects};
pub use model::{
    LoweredCleanupInstruction, LoweredCleanupPlan, LoweredLoopUnwindPlan, LoweredUnwindPlan,
    lower_cleanup_plans, lower_loop_unwind_plans, lower_return_unwind_plans,
};
pub(crate) use shared_lowering::lower_program_to_backend_module;
#[doc(hidden)]
pub use shared_lowering::lower_scalar_program;
pub(crate) use signature::lower_verb_with_expressions_as;
pub use signature::{
    BackendLayoutProvider, BackendSignature, lower_verb, lower_verb_signature,
    lower_verb_with_cleanup, lower_verb_with_expressions,
};
pub(crate) use signature::{lower_parameter, lower_return};
pub use symbols::{BackendSymbolBindings, SymbolError, SymbolIdentity, SymbolKind, SymbolRegistry};

#[cfg(feature = "cranelift")]
pub(crate) use cranelift::generic::expand_generic_instances;
#[cfg(feature = "cranelift")]
pub(crate) use cranelift::generic::specialize_program;
#[cfg(feature = "cranelift")]
pub(crate) use cranelift::generic::specialized_generic_name;
#[cfg(feature = "cranelift")]
pub(crate) use cranelift::native::reachable_call_names;
pub(crate) use result_constructors::normalize_program;
