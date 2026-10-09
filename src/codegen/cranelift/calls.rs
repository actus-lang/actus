mod arguments;
mod context;
mod lower;
mod method;
mod returns;

pub(crate) use context::CallLoweringContext;
pub(super) use lower::lower_call;
pub(super) use method::lower_method_call;
