mod body;
mod loops;
mod scopes;
mod statements;

use super::types::NativeType;
pub(super) use crate::codegen::model::LoweredCleanupSchedule;
pub(super) use body::{lower_body, lower_case_block};
pub(super) use statements::lower_statements;

#[derive(Clone, Copy)]
pub(crate) enum Flow {
    Fallthrough,
    Return(cranelift_codegen::ir::Value, Option<NativeType>),
    VoidReturn,
    Break,
    Continue,
}

#[derive(Clone)]
pub(crate) struct LoopTargets {
    pub(crate) header: cranelift_codegen::ir::Block,
    pub(crate) exit: cranelift_codegen::ir::Block,
    pub(crate) carried: Vec<String>,
}
