// Shared lowering receives explicit compiler contracts from both backends.
#![allow(clippy::too_many_arguments)]

mod aggregate_assignment;
mod constructors;
mod declarations;
mod expression;
mod functions;
mod layout;
mod orchestration;
mod performance;
mod returns;
mod runtime;
mod runtime_externals;
mod support;

pub(super) use super::{
    BackendBlockId, BackendCleanupArgument, BackendCleanupBridge, BackendCleanupBridgeDescriptor,
    BackendCleanupField, BackendCleanupPlan, BackendCleanupProvider,
    BackendCleanupResolvedDescriptor, BackendCleanupResult, BackendContractError, BackendExternal,
    BackendFunction, BackendInstruction, BackendLayout, BackendLayoutProvider, BackendModule,
    BackendParameter, BackendPassingMode, BackendReturn, BackendReturnMode, BackendSignature,
    BackendSymbolBindings, BackendTargetContext, BackendUnsupportedOperation, BackendValueType,
    LARGE_AGGREGATE_THRESHOLD, SymbolIdentity, SymbolKind, lower_parameter, lower_return,
    lower_semantic_cleanup_with_provider, lower_verb_with_expressions,
    lower_verb_with_expressions_as,
};
pub(super) use declarations::SharedDeclarations;
pub(super) use expression::SharedExpressionProvider;
pub(super) use functions::lower_shared_verb;
#[cfg(test)]
pub(super) use layout::ProgramLayouts;
#[cfg(test)]
pub(crate) use orchestration::lower_program_to_backend_module;
pub(crate) use orchestration::lower_program_to_backend_module_for_target_with_context;
#[cfg(test)]
pub(super) use orchestration::lower_program_to_backend_module_with_layouts;
pub use orchestration::lower_scalar_program;

#[cfg(test)]
mod tests;
