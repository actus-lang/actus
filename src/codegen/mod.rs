mod abi;
mod cleanup;
mod declarations;
mod expressions;
mod linker;
mod lowering;
mod model;
mod native;
mod native_runtime;
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
