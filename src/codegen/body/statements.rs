use super::super::{BackendContractError, BackendInstruction};
use super::provider::BackendExpressionProvider;
use crate::ast::{Expr, IntrinsicKind, lookup_call_intrinsic};
use crate::lexer::SourceSpan;

pub(super) fn lower_expression_statement(
    provider: &mut impl BackendExpressionProvider,
    expression: &Expr,
    _statement_span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(), BackendContractError> {
    if let Expr::Call { callee, arguments, span: _ } = expression {
        if lookup_call_intrinsic(callee).is_some_and(|kind| {
            matches!(
                kind,
                IntrinsicKind::Append
                    | IntrinsicKind::BufferLength
                    | IntrinsicKind::Crc32
                    | IntrinsicKind::Crc32Matches
                    | IntrinsicKind::ValidateFixedFrame
                    | IntrinsicKind::Print
            )
        }) {
            provider.lower_expression(expression, instructions).map(|_| ())?;
            return Ok(());
        }
        if provider.call_produces_value(callee) {
            provider.lower_expression(expression, instructions).map(|_| ())?;
            return Ok(());
        }
        provider.lower_unit_call(callee, arguments, _statement_span, instructions)?;
        return Ok(());
    }
    provider.lower_discarded_expression(expression, instructions)
}
