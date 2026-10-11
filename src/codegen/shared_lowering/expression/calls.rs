use super::super::super::{
    BackendContractError, BackendExpressionProvider, BackendInstruction, BackendValueId,
};
use super::super::support::expression_span;
use super::SharedExpressionProvider;
use crate::ast::Expr;

pub(super) fn lower_call(
    provider: &mut SharedExpressionProvider<'_>,
    expression: &Expr,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let Expr::Call { callee, arguments, span } = expression else {
        return Err(super::super::support::unsupported(
            expression_span(expression),
            "call expression expected",
        ));
    };
    if let Some(value) = super::layout::lower_intrinsic(provider, callee, *span, instructions)? {
        return Ok(value);
    }
    let arguments = arguments
        .iter()
        .map(|argument| provider.lower_expression(&argument.expression, instructions))
        .collect::<Result<Vec<_>, _>>()?;
    let result_type = provider.return_types.get(callee).copied().flatten();
    let result = result_type.map(|_| provider.allocate());
    if let (Some(result), Some(result_type)) = (result, result_type) {
        provider.remember_value(result, result_type);
        if let Some(payload_types) = provider.return_payload_types.get(callee) {
            for (discriminant, payload_type) in payload_types.iter().enumerate() {
                if let Some(payload_type) = payload_type {
                    provider.remember_enum_payload(result, discriminant as u32, *payload_type);
                }
            }
        }
    }
    instructions.push(BackendInstruction::Call {
        callee: provider.callee_for(callee, *span)?,
        arguments,
        result,
        span: *span,
    });
    result.ok_or_else(|| {
        super::super::support::unsupported(*span, "unit call cannot produce a value")
    })
}
