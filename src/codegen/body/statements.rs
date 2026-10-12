use super::super::{BackendContractError, BackendInstruction, BackendValueId};
use super::provider::BackendExpressionProvider;
use crate::ast::{Expr, IntrinsicKind, lookup_call_intrinsic};
use crate::lexer::SourceSpan;

pub(super) fn lower_expression_statement(
    provider: &mut impl BackendExpressionProvider,
    expression: &Expr,
    statement_span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(), BackendContractError> {
    let Expr::Call { callee, arguments, span } = expression else {
        return Err(super::super::body_contract::unsupported(
            statement_span,
            "expression statement",
        ));
    };
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
        provider.lower_expression(expression, instructions).map(|_| ())
    } else {
        let arguments = arguments
            .iter()
            .map(|argument| provider.lower_expression(&argument.expression, instructions))
            .collect::<Result<Vec<BackendValueId>, _>>()?;
        let callee = provider.callee_for(callee, *span)?;
        instructions.push(BackendInstruction::Call {
            callee,
            arguments,
            result: None,
            span: statement_span,
        });
        Ok(())
    }
}
