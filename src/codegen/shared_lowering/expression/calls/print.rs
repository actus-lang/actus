use crate::codegen::{
    BackendContractError, BackendInstruction, BackendValueId, BackendValueType, SymbolIdentity,
};
use crate::lexer::SourceSpan;

use super::super::SharedExpressionProvider;

pub(super) fn lower_print_call(
    provider: &mut SharedExpressionProvider<'_>,
    callee: &str,
    source_arguments: &[crate::ast::Argument],
    mut arguments: Vec<BackendValueId>,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let bool_index = if source_arguments.first().and_then(|argument| argument.name.as_deref())
        == Some("format")
    {
        Some(1)
    } else if source_arguments.first().and_then(|argument| argument.name.as_deref()) == Some("bool")
    {
        Some(0)
    } else {
        None
    };
    if let Some(index) = bool_index {
        normalize_bool_argument(provider, &mut arguments, index, span, instructions)?;
    }
    if source_arguments.first().and_then(|argument| argument.name.as_deref()) == Some("format") {
        let [format, value] = arguments.as_slice() else {
            return Err(super::super::super::support::unsupported(
                span,
                "formatted print requires a format and one value",
            ));
        };
        let bool_value =
            source_arguments.get(1).and_then(|argument| argument.name.as_deref()) == Some("bool");
        let stderr = callee == "eprint";
        let symbol = match (stderr, bool_value) {
            (false, false) => crate::runtime::PRINT_FORMAT_INT_SYMBOL,
            (false, true) => crate::runtime::PRINT_FORMAT_BOOL_SYMBOL,
            (true, false) => crate::runtime::PRINT_FORMAT_INT_STDERR_SYMBOL,
            (true, true) => crate::runtime::PRINT_FORMAT_BOOL_STDERR_SYMBOL,
        };
        return emit_print_call(provider, symbol, vec![*format, *value], span, instructions);
    }
    let [value] = arguments.as_slice() else {
        return Err(super::super::super::support::unsupported(span, "print requires one value"));
    };
    let label = source_arguments.first().and_then(|argument| argument.name.as_deref());
    let symbol = print_symbol(provider, callee, *value, label, span)?;
    emit_print_call(provider, symbol, arguments, span, instructions)
}

fn normalize_bool_argument(
    provider: &mut SharedExpressionProvider<'_>,
    arguments: &mut [BackendValueId],
    index: usize,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(), BackendContractError> {
    let Some(source) = arguments.get(index).copied() else {
        return Err(super::super::super::support::unsupported(
            span,
            "boolean output value missing",
        ));
    };
    if provider.value_type(source) != Some(BackendValueType::Bool) {
        return Ok(());
    }
    let target_type = BackendValueType::Integer { signed: true, width: 32 };
    let converted = provider.allocate();
    provider.remember_value(converted, target_type);
    instructions.push(BackendInstruction::IntegerCast {
        result: converted,
        source,
        source_type: BackendValueType::Bool,
        target_type,
        span,
    });
    arguments[index] = converted;
    Ok(())
}

fn emit_print_call(
    provider: &mut SharedExpressionProvider<'_>,
    symbol: &str,
    arguments: Vec<BackendValueId>,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let result = provider.allocate();
    let ty = BackendValueType::Integer { signed: true, width: 32 };
    provider.remember_value(result, ty);
    let callee = SymbolIdentity::external_raw(symbol)
        .map_err(|error| super::super::super::support::unsupported(span, error.to_string()))?;
    instructions.push(BackendInstruction::Call { callee, arguments, result: Some(result), span });
    Ok(result)
}

fn print_symbol(
    provider: &SharedExpressionProvider<'_>,
    callee: &str,
    value: BackendValueId,
    label: Option<&str>,
    span: SourceSpan,
) -> Result<&'static str, BackendContractError> {
    let stderr = callee == "eprint";
    if label == Some("bool") {
        return Ok(if stderr {
            crate::runtime::PRINT_BOOL_STDERR_SYMBOL
        } else {
            crate::runtime::PRINT_BOOL_SYMBOL
        });
    }
    match provider.value_type(value) {
        Some(BackendValueType::Integer { .. }) => Ok(if stderr {
            crate::runtime::PRINT_INT_STDERR_SYMBOL
        } else {
            crate::runtime::PRINT_INT_SYMBOL
        }),
        Some(BackendValueType::Pointer)
            if provider.nominal_type(value).is_some_and(|ty| ty.name == "Buffer") =>
        {
            Ok(if stderr {
                crate::runtime::PRINT_BUFFER_STDERR_SYMBOL
            } else {
                crate::runtime::PRINT_BUFFER_STDOUT_SYMBOL
            })
        }
        Some(BackendValueType::Pointer)
            if provider.nominal_type(value).is_some_and(|ty| ty.name == "String") =>
        {
            Ok(if stderr {
                crate::runtime::PRINT_STRING_STDERR_SYMBOL
            } else {
                crate::runtime::WRITE_STRING_STDOUT_SYMBOL
            })
        }
        Some(BackendValueType::Pointer) => Ok(if stderr {
            crate::runtime::PRINT_STRING_STDERR_SYMBOL
        } else {
            crate::runtime::PRINT_STRING_SYMBOL
        }),
        _ => Err(super::super::super::support::unsupported(span, "print value type")),
    }
}
