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
            return Err(super::super::super::errors::unsupported(
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
    let label = source_arguments.first().and_then(|argument| argument.name.as_deref());
    lower_value_print(provider, callee, label, arguments, span, instructions)
}

fn lower_value_print(
    provider: &mut SharedExpressionProvider<'_>,
    callee: &str,
    label: Option<&str>,
    mut arguments: Vec<BackendValueId>,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let [value] = arguments.as_slice() else {
        return Err(super::super::super::errors::unsupported(span, "print requires one value"));
    };
    let integer = match provider.value_type(*value) {
        Some(BackendValueType::Integer { signed, width }) => {
            Some(PrintInteger::for_type(signed, width, span)?)
        }
        _ => None,
    };
    let symbol = print_symbol(provider, callee, *value, label, integer, span)?;
    if let Some(integer) = integer {
        extend_integer_argument(provider, &mut arguments, integer, span, instructions);
    }
    emit_print_call(provider, symbol, arguments, span, instructions)
}

/// The runtime parameter an integer print argument is extended to. Values that
/// fit `i32` keep the original 32-bit entry point; every other integer goes
/// through the 64-bit signed or unsigned one.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PrintInteger {
    Int32,
    Signed64,
    Unsigned64,
}

impl PrintInteger {
    fn for_type(signed: bool, width: u8, span: SourceSpan) -> Result<Self, BackendContractError> {
        match (signed, width) {
            (_, 65..) => Err(super::super::super::errors::unsupported(
                span,
                "print of an integer wider than 64 bits",
            )),
            (true, ..=32) | (false, ..=31) => Ok(Self::Int32),
            (true, _) => Ok(Self::Signed64),
            (false, _) => Ok(Self::Unsigned64),
        }
    }

    fn runtime_type(self) -> BackendValueType {
        match self {
            Self::Int32 => BackendValueType::Integer { signed: true, width: 32 },
            Self::Signed64 => BackendValueType::Integer { signed: true, width: 64 },
            Self::Unsigned64 => BackendValueType::Integer { signed: false, width: 64 },
        }
    }
}

/// Sign- or zero-extends an integer print argument, by the signedness of its
/// own type, to the parameter of the chosen runtime print function.
fn extend_integer_argument(
    provider: &mut SharedExpressionProvider<'_>,
    arguments: &mut [BackendValueId],
    integer: PrintInteger,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) {
    let [source] = *arguments else { return };
    let Some(source_type) = provider.value_type(source) else { return };
    let target_type = integer.runtime_type();
    if source_type == target_type {
        return;
    }
    let widened = provider.allocate();
    provider.remember_value(widened, target_type);
    instructions.push(BackendInstruction::IntegerCast {
        result: widened,
        source,
        source_type,
        target_type,
        span,
    });
    arguments[0] = widened;
}

fn normalize_bool_argument(
    provider: &mut SharedExpressionProvider<'_>,
    arguments: &mut [BackendValueId],
    index: usize,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(), BackendContractError> {
    let Some(source) = arguments.get(index).copied() else {
        return Err(super::super::super::errors::unsupported(span, "boolean output value missing"));
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
        .map_err(|error| super::super::super::errors::unsupported(span, error.to_string()))?;
    instructions.push(BackendInstruction::Call { callee, arguments, result: Some(result), span });
    Ok(result)
}

fn print_symbol(
    provider: &SharedExpressionProvider<'_>,
    callee: &str,
    value: BackendValueId,
    label: Option<&str>,
    integer: Option<PrintInteger>,
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
        Some(BackendValueType::Integer { .. }) => Ok(match (integer, stderr) {
            (Some(PrintInteger::Signed64), false) => crate::runtime::PRINT_I64_SYMBOL,
            (Some(PrintInteger::Signed64), true) => crate::runtime::PRINT_I64_STDERR_SYMBOL,
            (Some(PrintInteger::Unsigned64), false) => crate::runtime::PRINT_U64_SYMBOL,
            (Some(PrintInteger::Unsigned64), true) => crate::runtime::PRINT_U64_STDERR_SYMBOL,
            (_, false) => crate::runtime::PRINT_INT_SYMBOL,
            (_, true) => crate::runtime::PRINT_INT_STDERR_SYMBOL,
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
        Some(BackendValueType::Pointer) => Ok(if stderr {
            crate::runtime::PRINT_STRING_STDERR_SYMBOL
        } else {
            crate::runtime::PRINT_STRING_SYMBOL
        }),
        _ => Err(super::super::super::errors::unsupported(span, "print value type")),
    }
}
