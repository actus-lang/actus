use crate::lexer::SourceSpan;

use super::super::backend::{
    BackendIntegerBinaryOp, BackendIntegerCompareOp, BackendIntegerUnaryOp,
};
use super::super::model::LoweredCleanupInstruction;
use super::super::symbols::SymbolIdentity;
use super::{BackendCleanupOperand, BackendFailure, BackendValueType};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendAggregateField {
    pub offset: u32,
    pub layout: super::BackendLayout,
    pub value: super::BackendValueId,
    pub ty: BackendValueType,
}

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
    IntegerCast {
        result: super::BackendValueId,
        source: super::BackendValueId,
        source_type: BackendValueType,
        target_type: BackendValueType,
        span: SourceSpan,
    },
    TrapIfZero {
        value: super::BackendValueId,
        reason: String,
        span: SourceSpan,
    },
    TrapIfSignedDivisionOverflow {
        left: super::BackendValueId,
        right: super::BackendValueId,
        width: u8,
        reason: String,
        span: SourceSpan,
    },
    TrapIfShiftCountOutOfRange {
        count: super::BackendValueId,
        width: u8,
        reason: String,
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
    EnumDiscriminant {
        result: super::BackendValueId,
        subject: super::BackendValueId,
        layout: super::BackendEnumLayout,
        span: SourceSpan,
    },
    EnumPayload {
        result: super::BackendValueId,
        subject: super::BackendValueId,
        layout: super::BackendEnumLayout,
        payload_type: BackendValueType,
        span: SourceSpan,
    },
    EnumPayloadBinding {
        result: super::BackendValueId,
        subject: super::BackendValueId,
        layout: super::BackendEnumLayout,
        payload_type: BackendValueType,
        mode: super::BackendCaseMode,
        span: SourceSpan,
    },
    EnumConstruct {
        result: super::BackendValueId,
        discriminant: u32,
        payload: super::BackendValueId,
        layout: super::BackendEnumLayout,
        payload_type: BackendValueType,
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
    AggregateConstruct {
        result: super::BackendValueId,
        layout: super::BackendLayout,
        fields: Vec<BackendAggregateField>,
        span: SourceSpan,
    },
    AggregateFieldLoad {
        result: super::BackendValueId,
        base: super::BackendValueId,
        aggregate_layout: super::BackendLayout,
        offset: u32,
        field_layout: super::BackendLayout,
        ty: BackendValueType,
        span: SourceSpan,
    },
    AggregateFieldStore {
        base: super::BackendValueId,
        aggregate_layout: super::BackendLayout,
        offset: u32,
        field_layout: super::BackendLayout,
        value: super::BackendValueId,
        ty: BackendValueType,
        span: SourceSpan,
    },
}
