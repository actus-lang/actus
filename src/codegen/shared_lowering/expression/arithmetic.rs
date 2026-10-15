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
        if matches!(operation, BackendIntegerBinaryOp::Divide | BackendIntegerBinaryOp::Remainder) {
            instructions.push(BackendInstruction::TrapIfZero {
                value: right,
                reason: "integer division or remainder by zero".to_owned(),
                span,
            });
        }
        if matches!(operation, BackendIntegerBinaryOp::Divide) {
            instructions.push(BackendInstruction::TrapIfSignedDivisionOverflow {
                left,
                right,
                width: operand_width,
                reason: "signed integer division overflow".to_owned(),
                span,
            });
        }
        if matches!(
            operation,
            BackendIntegerBinaryOp::ShiftLeft | BackendIntegerBinaryOp::ShiftRight
        ) {
            instructions.push(BackendInstruction::TrapIfShiftCountOutOfRange {
                count: right,
                width: operand_width,
                reason: "integer shift count exceeds operand width".to_owned(),
                span,
            });
        }
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
