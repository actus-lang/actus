use crate::lexer::SourceSpan;

use super::super::backend::{
    BackendIntegerBinaryOp, BackendIntegerCompareOp, BackendIntegerUnaryOp,
};
use super::super::model::LoweredCleanupInstruction;
use super::super::symbols::SymbolIdentity;
use super::{BackendCleanupOperand, BackendFailure, BackendValueType};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendInstruction {
    Constant {
        result: super::BackendValueId,
        ty: BackendValueType,
        value: u64,
        span: SourceSpan,
    },
    IntegerBinary {
        result: super::BackendValueId,
        operation: BackendIntegerBinaryOp,
        left: super::BackendValueId,
        right: super::BackendValueId,
        ty: BackendValueType,
        span: SourceSpan,
    },
    IntegerCompare {
        result: super::BackendValueId,
        operation: BackendIntegerCompareOp,
        left: super::BackendValueId,
        right: super::BackendValueId,
        signed: bool,
        span: SourceSpan,
    },
    IntegerUnary {
        result: super::BackendValueId,
        operation: BackendIntegerUnaryOp,
        operand: super::BackendValueId,
        ty: BackendValueType,
        span: SourceSpan,
    },
    Call {
        callee: SymbolIdentity,
        arguments: Vec<super::BackendValueId>,
        result: Option<super::BackendValueId>,
        span: SourceSpan,
    },
    Branch {
        condition: super::BackendValueId,
        then_block: super::BackendBlockId,
        else_block: super::BackendBlockId,
        span: SourceSpan,
    },
    Jump {
        target: super::BackendBlockId,
        span: SourceSpan,
    },
    Fail {
        failure: BackendFailure,
        span: SourceSpan,
    },
    Return {
        value: Option<super::BackendValueId>,
        span: SourceSpan,
    },
    Trap {
        reason: String,
        span: SourceSpan,
    },
    Cleanup {
        action: LoweredCleanupInstruction,
        operand: Option<BackendCleanupOperand>,
        span: SourceSpan,
    },
    MemoryCopy {
        destination: super::BackendValueId,
        source: super::BackendValueId,
        size: u32,
        alignment: u32,
        span: SourceSpan,
    },
}
