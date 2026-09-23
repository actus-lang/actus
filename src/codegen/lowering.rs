#[path = "lowering_body.rs"]
mod lowering_body;
#[path = "lowering_loops.rs"]
mod lowering_loops;
#[path = "lowering_scopes.rs"]
mod lowering_scopes;
#[path = "lowering_statements.rs"]
mod lowering_statements;

pub(super) use super::model::NativeCleanupSchedule;
pub(super) use lowering_body::{lower_body, lower_case_block};

#[derive(Clone, Copy)]
pub(super) enum Flow {
    Fallthrough,
    Return(cranelift_codegen::ir::Value),
    Break,
    Continue,
}

#[derive(Clone)]
pub(super) struct LoopTargets {
    pub(super) header: cranelift_codegen::ir::Block,
    pub(super) exit: cranelift_codegen::ir::Block,
    pub(super) carried: Vec<String>,
}
