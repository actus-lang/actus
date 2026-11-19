use crate::lexer::SourceSpan;

use super::super::super::{
    BackendContractError, BackendInstruction, BackendIntegerBinaryOp, BackendValueId,
    BackendValueType,
};
use super::SharedExpressionProvider;

impl SharedExpressionProvider<'_> {
    pub(super) fn lower_integer_binary_impl(
        &mut self,
        operation: BackendIntegerBinaryOp,
        left: BackendValueId,
        right: BackendValueId,
        span: SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
        // A shift count keeps its own type; every other operation takes both
        // operands at one type.
        let (left, right) = if matches!(
            operation,
            BackendIntegerBinaryOp::ShiftLeft | BackendIntegerBinaryOp::ShiftRight
        ) {
            (left, right)
        } else {
            super::state::unify_literal_operands(self, left, right, instructions)
        };
        let operand_width = match self.value_type(left) {
            Some(BackendValueType::Integer { width, .. }) => width,
            _ => 32,
        };
        let result_type = if matches!(
            operation,
            BackendIntegerBinaryOp::LogicalAnd | BackendIntegerBinaryOp::LogicalOr
        ) {
            BackendValueType::Bool
        } else {
            self.value_type(left).unwrap_or(BackendValueType::Integer { signed: true, width: 32 })
        };
        push_operation_traps(
            operation,
            left,
            right,
            result_type,
            operand_width,
            span,
            instructions,
        );
        let result = self.allocate();
        self.remember_value(result, result_type);
        instructions.push(BackendInstruction::IntegerBinary {
            result,
            operation,
            left,
            right,
            ty: result_type,
            span,
        });
        Ok(result)
    }
}

/// Division and remainder trap on a zero divisor, a signed division on
/// `MIN / -1`, and a shift on a count at or above the operand width.
fn push_operation_traps(
    operation: BackendIntegerBinaryOp,
    left: BackendValueId,
    right: BackendValueId,
    result_type: BackendValueType,
    operand_width: u8,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) {
    if matches!(operation, BackendIntegerBinaryOp::Divide | BackendIntegerBinaryOp::Remainder) {
        instructions.push(BackendInstruction::TrapIfZero {
            value: right,
            reason: "integer division or remainder by zero".to_owned(),
            span,
        });
    }
    // Only a signed division overflows (`MIN / -1`); an unsigned divisor
    // with all bits set is an ordinary value.
    if matches!(operation, BackendIntegerBinaryOp::Divide)
        && matches!(result_type, BackendValueType::Integer { signed: true, .. })
    {
        instructions.push(BackendInstruction::TrapIfSignedDivisionOverflow {
            left,
            right,
            width: operand_width,
            reason: "signed integer division overflow".to_owned(),
            span,
        });
    }
    if matches!(operation, BackendIntegerBinaryOp::ShiftLeft | BackendIntegerBinaryOp::ShiftRight) {
        instructions.push(BackendInstruction::TrapIfShiftCountOutOfRange {
            count: right,
            width: operand_width,
            reason: "integer shift count exceeds operand width".to_owned(),
            span,
        });
    }
}
