mod aggregates;
mod arena;
mod arrays;
mod branches;
mod calls;
mod cleanup;
mod context;
mod control_flow;
mod declarations;
mod emission;
mod enums;
mod failures;
mod float;
mod guards;
mod instruction;
mod integer;
mod literals;
mod memory;
mod module;
mod object;
mod packed;
mod returns;
mod scalar;
mod scalar_loan;
mod values;
mod verification;

pub use emission::{
    emit_module, emit_object, emit_object_bytes, emit_object_bytes_with_configuration,
    emit_object_with_configuration,
};
