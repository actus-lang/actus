pub(super) mod abi;
pub(super) mod aggregate;
pub(super) mod arenas;
pub(super) mod arrays;
pub(super) mod buffer_index;
pub(super) mod calls;
pub(super) mod case;
pub(super) mod cleanup;
pub(super) mod contract;
pub(super) mod control_flow;
pub(super) mod declarations;
pub(super) mod dependencies;
pub(super) mod dynamic_call;
pub(super) mod emission;
pub(super) mod emitter_declarations;
pub(super) mod enum_layout;
pub(super) mod enums;
pub(super) mod expressions;
pub(super) mod function_definition;
pub(super) mod generic;
mod integer_ops;
pub(super) mod ir_audit;
mod isa;
pub(super) mod layout;
pub(super) mod literals;
pub(super) mod lowering;
pub(super) mod native;
pub(super) mod object;
pub(super) mod performance;
pub(super) mod runtime;
pub(super) mod structs;
pub(super) mod types;
pub(super) mod vtable;

pub(super) use isa::build_isa_with_optimization;

pub use abi::{validate_external_native_signature, validate_native_signature};
pub use contract::emit_contract_module_object;
pub use native::{
    emit_module_object_for_target_in_namespace,
    emit_module_object_for_target_in_namespace_with_bindings,
    emit_module_object_for_target_in_namespace_with_bindings_and_instances,
    emit_module_object_for_target_in_namespace_with_bindings_and_instances_and_roots,
    emit_program_object, emit_program_object_for_target,
    emit_program_object_for_target_in_namespace,
    emit_program_object_for_target_in_namespace_with_bindings,
    emit_program_object_with_configuration, emit_zero_return_object,
};
