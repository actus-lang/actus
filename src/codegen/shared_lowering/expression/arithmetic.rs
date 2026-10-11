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
                width: 32,
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
                width: 32,
                reason: "integer shift count exceeds operand width".to_owned(),
                span,
            });
        }
        let result = self.allocate();
        self.remember_value(result, BackendValueType::Integer { signed: true, width: 32 });
        instructions.push(BackendInstruction::IntegerBinary {
            result,
            operation,
            left,
            right,
            ty: BackendValueType::Integer { signed: true, width: 32 },
            span,
        });
        Ok(result)
    }
}
