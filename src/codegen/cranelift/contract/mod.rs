mod aggregate;
mod arena;
mod arrays;
mod cleanup;
mod control_flow;
mod declarations;
mod dispatch;
mod emission;
mod enums;
mod functions;
mod packed;
mod returns;
mod scalar;

pub(super) use cleanup::LayoutCleanupProvider;
pub(crate) use declarations::{parameter_types, scalar_type};
pub use emission::emit_contract_module_object;
