use crate::ast::{CompoundAssignmentOp, Expr};
use crate::lexer::SourceSpan;

use super::super::super::{
    BackendContractError, BackendExpressionProvider, BackendInstruction, BackendIntegerBinaryOp,
};
use super::super::support::unsupported;
use super::SharedExpressionProvider;
use super::aggregate::lower_array_element_location;

pub(super) fn lower_array_element_compound_assignment(
    provider: &mut SharedExpressionProvider<'_>,
    collection: &Expr,
    index: &Expr,
    operator: CompoundAssignmentOp,
    value: &Expr,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(), BackendContractError> {
    let (base, index, aggregate_layout, element_layout, _, element_type, capacity) =
        lower_array_element_location(provider, collection, index, span, instructions)?;
    let current = provider.allocate();
    provider.remember_value(current, element_type);
    instructions.push(BackendInstruction::ArrayElementLoad {
        result: current,
        base,
        index,
        aggregate_layout,
        element_layout,
        element_type,
        capacity,
        span,
    });
    let value = provider.lower_expression(value, instructions)?;
    if provider.value_type(value) != Some(element_type) {
        return Err(unsupported(span, "array element compound assignment type mismatch"));
    }
    let result = provider.lower_integer_binary(
        compound_operation(operator),
        current,
        value,
        span,
        instructions,
    )?;
    instructions.push(BackendInstruction::ArrayElementStore {
        base,
        index,
        aggregate_layout,
        element_layout,
        element_type,
        value: result,
        capacity,
        span,
    });
    Ok(())
}

fn compound_operation(operator: CompoundAssignmentOp) -> BackendIntegerBinaryOp {
    match operator {
        CompoundAssignmentOp::Add => BackendIntegerBinaryOp::Add,
        CompoundAssignmentOp::Subtract => BackendIntegerBinaryOp::Subtract,
        CompoundAssignmentOp::Multiply => BackendIntegerBinaryOp::Multiply,
        CompoundAssignmentOp::Divide => BackendIntegerBinaryOp::Divide,
        CompoundAssignmentOp::Remainder => BackendIntegerBinaryOp::Remainder,
        CompoundAssignmentOp::BitwiseAnd => BackendIntegerBinaryOp::BitwiseAnd,
        CompoundAssignmentOp::BitwiseOr => BackendIntegerBinaryOp::BitwiseOr,
        CompoundAssignmentOp::BitwiseXor => BackendIntegerBinaryOp::BitwiseXor,
        CompoundAssignmentOp::ShiftLeft => BackendIntegerBinaryOp::ShiftLeft,
        CompoundAssignmentOp::ShiftRight => BackendIntegerBinaryOp::ShiftRight,
    }
}
