mod abi;
mod calls;
mod cleanup;
mod control_flow;
mod declarations;
mod enum_layout;
mod enums;
mod expression_construct;
mod expression_literals;
mod expression_operations;
mod expressions;
mod layout;
mod linker;
mod literals;
mod lowering;
mod model;
mod native;
mod native_runtime;
mod structs;
mod types;

pub use abi::{NativeAbiError, validate_external_native_signature, validate_native_signature};
pub use linker::{NativeLinkError, link_object};
pub use model::{
    NativeCleanupPlan, NativeInstruction, NativeLoopUnwindPlan, NativeUnwindPlan,
    lower_cleanup_plans, lower_loop_unwind_plans, lower_return_unwind_plans,
};
pub use native::{
    NativeEmitError, emit_program_object, emit_program_object_with_configuration,
    emit_zero_return_object,
};
