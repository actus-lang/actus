use crate::codegen::{
    BackendContractError, BackendInstruction, BackendValueId, BackendValueType, SymbolIdentity,
};
use crate::lexer::SourceSpan;

use super::super::SharedExpressionProvider;

pub(super) fn lower_print_call(
    provider: &mut SharedExpressionProvider<'_>,
    arguments: Vec<BackendValueId>,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let [value] = arguments.as_slice() else {
        return Err(super::super::super::support::unsupported(span, "print requires one value"));
    };
    let symbol = print_symbol(provider, *value, span)?;
    let callee = SymbolIdentity::external_raw(symbol)
        .map_err(|error| super::super::super::support::unsupported(span, error.to_string()))?;
    let result = provider.allocate();
    let ty = BackendValueType::Integer { signed: true, width: 32 };
    provider.remember_value(result, ty);
    instructions.push(BackendInstruction::Call { callee, arguments, result: Some(result), span });
    Ok(result)
}

fn print_symbol(
    provider: &SharedExpressionProvider<'_>,
    value: BackendValueId,
    span: SourceSpan,
) -> Result<&'static str, BackendContractError> {
    match provider.value_type(value) {
        Some(BackendValueType::Integer { .. }) => Ok(crate::runtime::PRINT_INT_SYMBOL),
        Some(BackendValueType::Pointer)
            if provider.nominal_type(value).is_some_and(|ty| ty.name == "Buffer") =>
        {
            Ok(crate::runtime::PRINT_BUFFER_STDOUT_SYMBOL)
        }
        Some(BackendValueType::Pointer)
            if provider.nominal_type(value).is_some_and(|ty| ty.name == "String") =>
        {
            Ok(crate::runtime::WRITE_STRING_STDOUT_SYMBOL)
        }
        Some(BackendValueType::Pointer) => Ok(crate::runtime::PRINT_STRING_SYMBOL),
        _ => Err(super::super::super::support::unsupported(span, "print value type")),
    }
}
