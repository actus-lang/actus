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
        .map_err(|error| super::super::super::support::unsupported(span, error.to_string()))?;
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
    if lookup_call_intrinsic(callee) != Some(IntrinsicKind::Append) {
        return Ok(arguments);
    }
    let byte = arguments
        .get(1)
        .copied()
        .ok_or_else(|| super::super::super::support::unsupported(span, "append requires a byte"))?;
    let source_type = provider.value_type(byte).ok_or_else(|| {
        super::super::super::support::unsupported(span, "append byte type is unavailable")
    })?;
    let target_type = BackendValueType::Integer { signed: false, width: 8 };
    if source_type != target_type {
        let converted = provider.allocate();
        provider.remember_value(converted, target_type);
        instructions.push(BackendInstruction::IntegerCast {
            result: converted,
            source: byte,
            source_type,
            target_type,
            span,
        });
        arguments[1] = converted;
    }
    Ok(arguments)
}
