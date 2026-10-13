use super::super::super::{
    BackendContractError, BackendExpressionProvider, BackendInstruction, BackendValueId,
};
use super::super::support::expression_span;
use super::SharedExpressionProvider;
use crate::ast::{Expr, IntrinsicKind, lookup_call_intrinsic};
use crate::codegen::{SymbolIdentity, SymbolKind};
use crate::lexer::SourceSpan;

pub(super) fn callee_for(
    provider: &SharedExpressionProvider<'_>,
    callee: &str,
    span: SourceSpan,
) -> Result<SymbolIdentity, BackendContractError> {
    provider
        .external_symbols
        .get(callee)
        .cloned()
        .or_else(|| {
            provider
                .bindings
                .map(|bindings| bindings.external_symbol(provider.namespace, callee))
                .filter(|symbol| *symbol != callee)
                .and_then(|symbol| SymbolIdentity::external_raw(symbol).ok())
        })
        .or_else(|| {
            provider
                .return_types
                .contains_key(callee)
                .then(|| SymbolIdentity::new(provider.namespace, SymbolKind::Verb, callee))
                .and_then(Result::ok)
        })
        .map_or_else(|| SymbolIdentity::new(provider.namespace, SymbolKind::Verb, callee), Ok)
        .map_err(|error| super::super::support::unsupported(span, error.to_string()))
}

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
    if arguments.is_empty() && callee.starts_with("Array[") {
        return lower_array_constructor(provider, callee, *span, instructions);
    }
    if arguments.is_empty() && callee.starts_with("Arena[") {
        return super::arena::lower_constructor(provider, callee, *span, instructions);
    }
    let arguments = lower_arguments(provider, arguments, instructions)?;
    if let Some((symbol, result_type)) = runtime_intrinsic(callee) {
        return lower_runtime_call(
            provider,
            callee,
            symbol,
            result_type,
            arguments,
            *span,
            instructions,
        );
    }
    if lookup_call_intrinsic(callee) == Some(IntrinsicKind::Print) {
        return lower_print_call(provider, arguments, *span, instructions);
    }
    let result_type = provider.return_types.get(callee).copied().flatten();
    let result = result_type.map(|_| provider.allocate());
    if let (Some(result), Some(result_type)) = (result, result_type) {
        provider.remember_value(result, result_type);
        remember_return_metadata(provider, result, callee);
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

fn remember_return_metadata(
    provider: &mut SharedExpressionProvider<'_>,
    result: BackendValueId,
    callee: &str,
) {
    if let Some(Some(nominal_type)) = provider.return_nominal_types.get(callee) {
        provider.remember_nominal_type(result, nominal_type.clone());
    }
    if let Some(payload_types) = provider.return_payload_types.get(callee) {
        for (discriminant, payload_type) in payload_types.iter().enumerate() {
            if let Some(payload_type) = payload_type {
                provider.remember_enum_payload(result, discriminant as u32, *payload_type);
            }
        }
    }
}

pub(super) fn lower_unit_call(
    provider: &mut SharedExpressionProvider<'_>,
    expression: &Expr,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(), BackendContractError> {
    let Expr::Call { callee, arguments, span } = expression else {
        return Err(super::super::support::unsupported(
            expression_span(expression),
            "unit call expected",
        ));
    };
    let arguments = lower_arguments(provider, arguments, instructions)?;
    instructions.push(BackendInstruction::Call {
        callee: provider.callee_for(callee, *span)?,
        arguments,
        result: None,
        span: *span,
    });
    Ok(())
}

fn lower_arguments(
    provider: &mut SharedExpressionProvider<'_>,
    arguments: &[crate::ast::Argument],
    instructions: &mut Vec<BackendInstruction>,
) -> Result<Vec<BackendValueId>, BackendContractError> {
    arguments
        .iter()
        .map(|argument| provider.lower_expression(&argument.expression, instructions))
        .collect()
}

fn lower_runtime_call(
    provider: &mut SharedExpressionProvider<'_>,
    callee: &str,
    symbol: &str,
    result_type: crate::codegen::BackendValueType,
    arguments: Vec<BackendValueId>,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let arguments = normalize_runtime_arguments(provider, callee, arguments, span, instructions)?;
    let result = provider.allocate();
    provider.remember_value(result, result_type);
    instructions.push(BackendInstruction::Call {
        callee: SymbolIdentity::external_raw(symbol)
            .map_err(|error| super::super::support::unsupported(span, error.to_string()))?,
        arguments,
        result: Some(result),
        span,
    });
    Ok(result)
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
        .ok_or_else(|| super::super::support::unsupported(span, "append requires a byte"))?;
    let source_type = provider.value_type(byte).ok_or_else(|| {
        super::super::support::unsupported(span, "append byte type is unavailable")
    })?;
    let target_type = crate::codegen::BackendValueType::Integer { signed: false, width: 8 };
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

fn runtime_intrinsic(callee: &str) -> Option<(&'static str, crate::codegen::BackendValueType)> {
    match lookup_call_intrinsic(callee) {
        Some(IntrinsicKind::Append) => Some((
            crate::runtime::BUFFER_APPEND_SYMBOL,
            crate::codegen::BackendValueType::Integer { signed: false, width: 8 },
        )),
        Some(IntrinsicKind::BufferLength) => Some((
            crate::runtime::BUFFER_LENGTH_SYMBOL,
            crate::codegen::BackendValueType::Integer { signed: true, width: 32 },
        )),
        _ => None,
    }
}

fn lower_print_call(
    provider: &mut SharedExpressionProvider<'_>,
    arguments: Vec<BackendValueId>,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let [value] = arguments.as_slice() else {
        return Err(super::super::support::unsupported(span, "print requires one value"));
    };
    let symbol = match provider.value_type(*value) {
        Some(crate::codegen::BackendValueType::Integer { .. }) => crate::runtime::PRINT_INT_SYMBOL,
        Some(crate::codegen::BackendValueType::Pointer)
            if provider.nominal_type(*value).is_some_and(|ty| ty.name == "Buffer") =>
        {
            crate::runtime::PRINT_BUFFER_STDOUT_SYMBOL
        }
        Some(crate::codegen::BackendValueType::Pointer) => crate::runtime::PRINT_STRING_SYMBOL,
        _ => return Err(super::super::support::unsupported(span, "print value type")),
    };
    let callee = SymbolIdentity::external_raw(symbol)
        .map_err(|error| super::super::support::unsupported(span, error.to_string()))?;
    let result = provider.allocate();
    let ty = crate::codegen::BackendValueType::Integer { signed: true, width: 32 };
    provider.remember_value(result, ty);
    instructions.push(BackendInstruction::Call { callee, arguments, result: Some(result), span });
    Ok(result)
}

fn lower_array_constructor(
    provider: &mut SharedExpressionProvider<'_>,
    callee: &str,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let type_name = crate::semantic::parse_type_name_key(callee, span).ok_or_else(|| {
        super::super::support::unsupported(span, "malformed fixed-array constructor")
    })?;
    if type_name.name != "Array" || type_name.arguments.len() != 2 {
        return Err(super::super::support::unsupported(
            span,
            "fixed-array constructor requires Array[T, N]",
        ));
    }
    let capacity = type_name.arguments[1]
        .name
        .parse::<u32>()
        .map_err(|_| super::super::support::unsupported(span, "array capacity is not concrete"))?;
    if capacity == 0 {
        return Err(super::super::support::unsupported(
            span,
            "fixed-array capacity must be non-zero",
        ));
    }
    let layout = provider
        .layouts
        .layout_for(&type_name)
        .ok_or_else(|| super::super::support::unsupported(span, "array has no neutral layout"))?;
    let element_layout = provider.layouts.layout_for(&type_name.arguments[0]).ok_or_else(|| {
        super::super::support::unsupported(span, "array element has no neutral layout")
    })?;
    if u64::from(element_layout.size) * u64::from(capacity) != u64::from(layout.size) {
        return Err(super::super::support::unsupported(
            span,
            "array layout does not match capacity",
        ));
    }
    let result = provider.allocate();
    provider.remember_value(result, super::super::super::BackendValueType::Aggregate { layout });
    provider.remember_nominal_type(result, type_name);
    instructions.push(BackendInstruction::AggregateConstruct {
        result,
        layout,
        fields: Vec::new(),
        span,
    });
    Ok(result)
}
