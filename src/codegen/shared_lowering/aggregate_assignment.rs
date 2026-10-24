use crate::lexer::SourceSpan;

use super::super::{
    BackendContractError, BackendInstruction, BackendLayout, BackendValueId, SymbolIdentity,
};
use super::expression::SharedExpressionProvider;
use super::support::unsupported;

pub(super) fn assign_aggregate_loan(
    provider: &mut SharedExpressionProvider<'_>,
    destination: BackendValueId,
    value: BackendValueId,
    layout: BackendLayout,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(), BackendContractError> {
    let destination_is_buffer =
        provider.nominal_type(destination).is_some_and(|type_name| type_name.name == "Buffer");
    let source_is_buffer =
        provider.nominal_type(value).is_some_and(|type_name| type_name.name == "Buffer");
    if destination_is_buffer && source_is_buffer {
        instructions.push(BackendInstruction::Call {
            callee: SymbolIdentity::external_raw(crate::runtime::BUFFER_MOVE_SYMBOL)
                .map_err(|error| unsupported(span, error.to_string()))?,
            arguments: vec![destination, value],
            result: None,
            span,
        });
    } else {
        instructions.push(BackendInstruction::MemoryCopy {
            destination,
            source: value,
            size: layout.size,
            alignment: layout.alignment,
            span,
        });
    }
    Ok(())
}
