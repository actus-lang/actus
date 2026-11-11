use crate::ast::{IntrinsicKind, lookup_call_intrinsic};
use crate::codegen::{
    BackendContractError, BackendInstruction, BackendValueId, BackendValueType, SymbolIdentity,
};
use crate::lexer::SourceSpan;

use super::super::SharedExpressionProvider;

pub(super) fn lower_runtime_call(
    provider: &mut SharedExpressionProvider<'_>,
    callee: &str,
    symbol: &str,
    result_type: BackendValueType,
    arguments: Vec<BackendValueId>,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let arguments = normalize_runtime_arguments(provider, callee, arguments, span, instructions)?;
    let result = provider.allocate();
    provider.remember_value(result, result_type);
    let callee = SymbolIdentity::external_raw(symbol)
        .map_err(|error| super::super::super::errors::unsupported(span, error.to_string()))?;
    instructions.push(BackendInstruction::Call { callee, arguments, result: Some(result), span });
    Ok(result)
}

pub(super) fn runtime_intrinsic(callee: &str) -> Option<(&'static str, BackendValueType)> {
    match lookup_call_intrinsic(callee) {
        Some(IntrinsicKind::Append) => Some((
            crate::runtime::BUFFER_APPEND_SYMBOL,
            BackendValueType::Integer { signed: false, width: 8 },
        )),
        Some(IntrinsicKind::BufferLength) => Some((
            crate::runtime::BUFFER_LENGTH_SYMBOL,
            BackendValueType::Integer { signed: true, width: 32 },
        )),
        Some(IntrinsicKind::Crc32) => Some((
            crate::runtime::BUFFER_CRC32_SYMBOL,
            BackendValueType::Integer { signed: true, width: 64 },
        )),
        Some(IntrinsicKind::Crc32Matches) => Some((
            crate::runtime::BUFFER_CRC32_MATCHES_SYMBOL,
            BackendValueType::Integer { signed: true, width: 64 },
        )),
        Some(IntrinsicKind::ValidateFixedFrame) => Some((
            crate::runtime::BUFFER_VALIDATE_FIXED_FRAME_SYMBOL,
            BackendValueType::Integer { signed: true, width: 64 },
        )),
        _ => None,
    }
}

fn normalize_runtime_arguments(
    provider: &mut SharedExpressionProvider<'_>,
    callee: &str,
    mut arguments: Vec<BackendValueId>,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<Vec<BackendValueId>, BackendContractError> {
    match lookup_call_intrinsic(callee) {
        Some(IntrinsicKind::Append) => {
            let byte = arguments.get(1).copied().ok_or_else(|| {
                super::super::super::errors::unsupported(span, "append requires a byte")
            })?;
            arguments[1] = coerce_runtime_integer(
                provider,
                byte,
                BackendValueType::Integer { signed: false, width: 8 },
                span,
                instructions,
                "append byte type is unavailable",
            )?;
        }
        Some(IntrinsicKind::Crc32)
        | Some(IntrinsicKind::Crc32Matches)
        | Some(IntrinsicKind::ValidateFixedFrame) => {
            for argument in arguments.iter_mut().skip(1) {
                *argument = coerce_runtime_integer(
                    provider,
                    *argument,
                    BackendValueType::Integer { signed: true, width: 64 },
                    span,
                    instructions,
                    "runtime serialization argument type is unavailable",
                )?;
            }
        }
        _ => {}
    }
    Ok(arguments)
}

fn coerce_runtime_integer(
    provider: &mut SharedExpressionProvider<'_>,
    value: BackendValueId,
    target_type: BackendValueType,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
    missing_type_message: &'static str,
) -> Result<BackendValueId, BackendContractError> {
    let source_type = provider
        .value_type(value)
        .ok_or_else(|| super::super::super::errors::unsupported(span, missing_type_message))?;
    if source_type == target_type {
        return Ok(value);
    }
    let converted = provider.allocate();
    provider.remember_value(converted, target_type);
    instructions.push(BackendInstruction::IntegerCast {
        result: converted,
        source: value,
        source_type,
        target_type,
        span,
    });
    Ok(converted)
}
