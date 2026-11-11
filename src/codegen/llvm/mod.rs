mod aggregate;
mod arena;
mod branches;
mod calls;
mod cleanup;
mod context;
mod control_flow;
mod declarations;
mod emission;
mod enums;
mod failures;
mod guards;
mod identity;
mod instruction;
mod literals;
mod memory;
mod module;
mod object;
mod returns;
mod scalar;
mod shim;
mod values;
mod verification;

pub use identity::{LlvmIdentity, ToolchainIdentity, strict_identity_error, toolchain_identity};
pub use object::{StackSizeParseError, StackSizeRecord, parse_stack_sizes};

pub use emission::{
    ObjectEmissionOptions, ObjectOptimizationLevel, emit_module, emit_object, emit_object_bytes,
    emit_object_bytes_with_configuration, emit_object_bytes_with_configuration_and_float_policy,
    emit_object_bytes_with_configuration_and_options_and_float_policy,
    emit_object_with_configuration, emit_object_with_configuration_and_options,
};
