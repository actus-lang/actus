use super::*;

impl SharedExpressionProvider<'_> {
    pub(super) fn lower_compare(
        &mut self,
        left: BackendValueId,
        right: BackendValueId,
        operator: BinaryOp,
        span: SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
        let operation = match operator {
            BinaryOp::LessThan => BackendIntegerCompareOp::LessThan,
            BinaryOp::LessEquals => BackendIntegerCompareOp::LessEquals,
            BinaryOp::GreaterThan => BackendIntegerCompareOp::GreaterThan,
            BinaryOp::GreaterEquals => BackendIntegerCompareOp::GreaterEquals,
            BinaryOp::Equals => BackendIntegerCompareOp::Equals,
            BinaryOp::NotEquals => BackendIntegerCompareOp::NotEquals,
            _ => return Err(unsupported(span, "comparison operator expected")),
        };
        let left_type = self
            .value_type(left)
            .ok_or_else(|| unsupported(span, "left comparison operand type is unavailable"))?;
        let right_type = self
            .value_type(right)
            .ok_or_else(|| unsupported(span, "right comparison operand type is unavailable"))?;
        if let (
            BackendValueType::Float { width: left_width },
            BackendValueType::Float { width: right_width },
        ) = (left_type, right_type)
        {
            if left_width != right_width {
                return Err(unsupported(span, "floating-point comparison widths must match"));
            }
            let result = self.allocate();
            self.remember_value(result, BackendValueType::Bool);
            instructions.push(BackendInstruction::FloatCompare {
                result,
                operation,
                left,
                right,
                ty: left_type,
                span,
            });
            return Ok(result);
        }
        let result = self.allocate();
        self.remember_value(result, BackendValueType::Bool);
        instructions.push(BackendInstruction::IntegerCompare {
            result,
            operation,
            left,
            right,
            signed: matches!(left_type, BackendValueType::Integer { signed: true, .. }),
            span,
        });
        Ok(result)
    }
}
