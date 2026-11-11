mod cleanup;
mod dominance;
mod errors;
mod flow;
mod instruction;
mod semantic_cleanup;
mod types;
mod validation;

use super::abi::BackendEnumLayout;

pub use super::backend::BackendIntegerBinaryOp;
pub use super::backend::BackendIntegerCompareOp;
pub use super::backend::BackendIntegerUnaryOp;
pub use cleanup::{
    BackendCleanupArgument, BackendCleanupBridge, BackendCleanupBridgeDescriptor,
    BackendCleanupField, BackendCleanupOperand, BackendCleanupProvider,
    BackendCleanupResolvedDescriptor, BackendCleanupResult,
};
pub use errors::{BackendContractError, BackendTargetContext, BackendUnsupportedOperation};
pub use flow::{BackendBlockParameter, BackendEdgeArgument, validate_edge_arguments};
pub use instruction::{BackendAggregateField, BackendInstruction};
pub use semantic_cleanup::lower_semantic_cleanup_with_provider;
pub(crate) use types::reject_float_ir;
pub(super) use types::validate_value_type;
pub use types::{
    BackendAbiError, BackendBlock, BackendBlockId, BackendCaseMode, BackendCleanupAction,
    BackendCleanupOrigin, BackendCleanupPlan, BackendEmitError, BackendExternal, BackendFailure,
    BackendFunction, BackendLayout, BackendModule, BackendParameter, BackendPassingMode,
    BackendReturn, BackendReturnMode, BackendValueId, BackendValueType, lower_semantic_cleanup,
};
