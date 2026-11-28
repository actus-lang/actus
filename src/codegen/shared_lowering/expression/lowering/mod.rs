use super::super::super::{
    BackendContractError, BackendExpressionProvider, BackendInstruction, BackendIntegerBinaryOp,
    BackendIntegerUnaryOp, BackendValueId, BackendValueType, SymbolIdentity,
};
use super::super::errors::{expression_span, unsupported};
use super::SharedExpressionProvider;
use super::{aggregate, arena, array_ops, calls, case, cleanup, dispatch, state};
use crate::ast::{CaseMode, Expr, UnaryOp, VariantPayload};
use crate::lexer::SourceSpan;
use crate::semantic::LoopExitKind;
use std::collections::BTreeMap;

mod inherent;
mod provider;

pub(in crate::codegen::shared_lowering) use inherent::lower_unit_call;
