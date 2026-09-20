mod model;
mod native;

pub use model::{NativeCleanupPlan, NativeInstruction, lower_cleanup_plans};
pub use native::{NativeEmitError, emit_program_object, emit_zero_return_object};
