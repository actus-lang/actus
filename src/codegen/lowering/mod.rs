mod body;
mod loops;
mod scopes;
mod statements;

pub(super) use super::model::NativeCleanupSchedule;
pub(super) use body::{lower_body, lower_case_block};

#[derive(Clone, Copy)]
pub(super) enum Flow {
    Fallthrough,
    Return(cranelift_codegen::ir::Value),
    VoidReturn,
    Break,
    Continue,
}

#[derive(Clone)]
pub(super) struct LoopTargets {
    pub(super) header: cranelift_codegen::ir::Block,
    pub(super) exit: cranelift_codegen::ir::Block,
    pub(super) carried: Vec<String>,
}
