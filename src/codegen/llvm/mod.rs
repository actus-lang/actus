mod calls;
mod context;
mod control_flow;
mod declarations;
mod emission;
mod guards;
mod integer_ops;
mod object;
mod returns;
mod values;

pub use emission::{emit_module, emit_object, emit_object_bytes};
