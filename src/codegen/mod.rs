mod abi;
mod arenas;
mod arrays;
mod calls;
mod case;
mod cleanup;
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
mod structs;
mod target;
mod types;
mod vtable;

pub use abi::{NativeAbiError, validate_external_native_signature, validate_native_signature};
pub use linker::{NativeLinkError, link_object};
pub use model::{
    NativeCleanupPlan, NativeInstruction, NativeLoopUnwindPlan, NativeUnwindPlan,
    lower_cleanup_plans, lower_loop_unwind_plans, lower_return_unwind_plans,
};
pub use native::{
    NativeEmitError, emit_program_object, emit_program_object_for_target,
    emit_program_object_with_configuration, emit_zero_return_object,
};
