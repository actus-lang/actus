use super::super::super::{
    BackendContractError, BackendExpressionProvider, BackendInstruction, BackendValueId,
};
use super::super::support::expression_span;
use super::SharedExpressionProvider;
use crate::ast::Expr;
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
                .return_types
                .contains_key(callee)
                .then(|| SymbolIdentity::new(provider.namespace, SymbolKind::Verb, callee))
                .and_then(Result::ok)
        })
        .or_else(|| {
            provider
                .bindings
                .map(|bindings| bindings.external_symbol(provider.namespace, callee))
                .filter(|symbol| *symbol != callee)
                .and_then(|symbol| SymbolIdentity::external_raw(symbol).ok())
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
    let arguments = arguments
        .iter()
        .map(|argument| provider.lower_expression(&argument.expression, instructions))
        .collect::<Result<Vec<_>, _>>()?;
    let result_type = provider.return_types.get(callee).copied().flatten();
    let result = result_type.map(|_| provider.allocate());
    if let (Some(result), Some(result_type)) = (result, result_type) {
        provider.remember_value(result, result_type);
        if let Some(payload_types) = provider.return_payload_types.get(callee) {
            for (discriminant, payload_type) in payload_types.iter().enumerate() {
                if let Some(payload_type) = payload_type {
                    provider.remember_enum_payload(result, discriminant as u32, *payload_type);
                }
            }
        }
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
