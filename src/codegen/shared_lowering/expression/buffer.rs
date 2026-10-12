use crate::ast::Expr;
use crate::lexer::SourceSpan;

use super::super::super::{
    BackendContractError, BackendInstruction, BackendValueId, BackendValueType,
};
use super::super::support::unsupported;
use super::SharedExpressionProvider;
use crate::codegen::BackendExpressionProvider;

pub(super) fn lower_buffer_element_from_base(
    provider: &mut SharedExpressionProvider<'_>,
    base: BackendValueId,
    index: &Expr,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let index = provider.lower_expression(index, instructions)?;
    let index_type = provider
        .value_type(index)
        .ok_or_else(|| unsupported(span, "Buffer index type is unavailable"))?;
    let index_i32 = if index_type == (BackendValueType::Integer { signed: true, width: 32 }) {
        index
    } else {
        let converted = provider.allocate();
        let target_type = BackendValueType::Integer { signed: true, width: 32 };
        provider.remember_value(converted, target_type);
        instructions.push(BackendInstruction::IntegerCast {
            result: converted,
            source: index,
            source_type: index_type,
            target_type,
            span,
        });
        converted
    };
    let raw = provider.allocate();
    let raw_type = BackendValueType::Integer { signed: true, width: 32 };
    provider.remember_value(raw, raw_type);
    instructions.push(BackendInstruction::Call {
        callee: crate::codegen::SymbolIdentity::external_raw(crate::runtime::BUFFER_BYTE_AT_SYMBOL)
            .map_err(|error| unsupported(span, error.to_string()))?,
        arguments: vec![base, index_i32],
        result: Some(raw),
        span,
    });
    let result = provider.allocate();
    let result_type = BackendValueType::Integer { signed: false, width: 8 };
    provider.remember_value(result, result_type);
    instructions.push(BackendInstruction::IntegerCast {
        result,
        source: raw,
        source_type: raw_type,
        target_type: result_type,
        span,
    });
    Ok(result)
}

pub(super) fn lower_buffer_element_assignment_from_base(
    provider: &mut SharedExpressionProvider<'_>,
    base: BackendValueId,
    index: &Expr,
    value: &Expr,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(), BackendContractError> {
    let index = provider.lower_expression(index, instructions)?;
    let value = provider.lower_expression(value, instructions)?;
    let index = checked_i32(provider, index, span, instructions)?;
    let value_type = provider
        .value_type(value)
        .ok_or_else(|| unsupported(span, "Buffer byte value type is unavailable"))?;
    let byte_type = BackendValueType::Integer { signed: false, width: 8 };
    if value_type != byte_type {
        return Err(unsupported(span, "Buffer byte assignment requires u8"));
    }
    let result = provider.allocate();
    provider.remember_value(result, BackendValueType::Integer { signed: true, width: 32 });
    instructions.push(BackendInstruction::Call {
        callee: crate::codegen::SymbolIdentity::external_raw(
            crate::runtime::BUFFER_BYTE_SET_SYMBOL,
        )
        .map_err(|error| unsupported(span, error.to_string()))?,
        arguments: vec![base, index, value],
        result: Some(result),
        span,
    });
    Ok(())
}

fn checked_i32(
    provider: &mut SharedExpressionProvider<'_>,
    value: BackendValueId,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let source_type = provider
        .value_type(value)
        .ok_or_else(|| unsupported(span, "Buffer index type is unavailable"))?;
    let target_type = BackendValueType::Integer { signed: true, width: 32 };
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
