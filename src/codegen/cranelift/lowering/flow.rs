use super::super::types::NativeType;

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
