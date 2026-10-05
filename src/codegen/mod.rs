mod abi;
mod arenas;
mod arrays;
mod buffer_index;
mod calls;
mod case;
mod cleanup;
mod constants;
mod control_flow;
mod declarations;
mod dynamic_call;
mod enum_layout;
mod enums;
mod expressions;
mod function_definition;
mod generic;
mod layout;
mod linker;
mod literals;
mod lowering;
mod model;
mod native;
mod native_runtime;
mod performance;
mod result_constructors;
mod serialization_generation;
mod serialization_migration;
mod structs;
mod symbols;
mod target;
mod types;
mod vtable;

pub use abi::{NativeAbiError, validate_external_native_signature, validate_native_signature};
pub use linker::{NativeLinkError, link_object, link_objects};
pub use model::{
    NativeCleanupPlan, NativeInstruction, NativeLoopUnwindPlan, NativeUnwindPlan,
    lower_cleanup_plans, lower_loop_unwind_plans, lower_return_unwind_plans,
};
pub use native::{
    NativeEmitError, NativeSymbolBindings, emit_module_object_for_target_in_namespace,
    emit_module_object_for_target_in_namespace_with_bindings,
    emit_module_object_for_target_in_namespace_with_bindings_and_instances,
    emit_module_object_for_target_in_namespace_with_bindings_and_instances_and_roots,
    emit_program_object, emit_program_object_for_target,
    emit_program_object_for_target_in_namespace,
    emit_program_object_for_target_in_namespace_with_bindings,
    emit_program_object_with_configuration, emit_zero_return_object,
};
pub use symbols::{SymbolError, SymbolIdentity, SymbolKind, SymbolRegistry};

pub(crate) use generic::expand_generic_instances;
pub(crate) use generic::specialized_generic_name;
pub(crate) use result_constructors::normalize_program;
