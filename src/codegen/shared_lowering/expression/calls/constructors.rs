use crate::ast::TypeName;
use crate::codegen::{BackendContractError, BackendInstruction, BackendValueId, BackendValueType};
use crate::lexer::SourceSpan;

use super::super::SharedExpressionProvider;

pub(super) fn lower_array_constructor(
    provider: &mut SharedExpressionProvider<'_>,
    callee: &str,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let type_name = crate::semantic::parse_type_name_key(callee, span).ok_or_else(|| {
        super::super::super::support::unsupported(span, "malformed fixed-array constructor")
    })?;
    validate_array_constructor(&type_name, span)?;
    let capacity = parse_capacity(&type_name, span)?;
    let element_layout = provider.layouts.layout_for(&type_name.arguments[0]).ok_or_else(|| {
        super::super::super::support::unsupported(span, "array element has no neutral layout")
    })?;
    let size = element_layout.size.checked_mul(capacity).ok_or_else(|| {
        super::super::super::support::unsupported(span, "array layout size overflow")
    })?;
    let layout = provider.layouts.layout_for(&type_name).ok_or_else(|| {
        super::super::super::support::unsupported(span, "array has no neutral layout")
    })?;
    if size != layout.size {
        return Err(super::super::super::support::unsupported(
            span,
            "array layout does not match capacity",
        ));
    }
    let result = provider.allocate();
    provider.remember_value(result, BackendValueType::Aggregate { layout });
    provider.remember_nominal_type(result, type_name);
    instructions.push(BackendInstruction::AggregateConstruct {
        result,
        layout,
        fields: Vec::new(),
        span,
    });
    Ok(result)
}

fn validate_array_constructor(
    type_name: &TypeName,
    span: SourceSpan,
) -> Result<(), BackendContractError> {
    if type_name.name != "Array" || type_name.arguments.len() != 2 {
        return Err(super::super::super::support::unsupported(
            span,
            "fixed-array constructor requires Array[T, N]",
        ));
    }
    Ok(())
}

fn parse_capacity(type_name: &TypeName, span: SourceSpan) -> Result<u32, BackendContractError> {
    let capacity = type_name.arguments[1].name.parse::<u32>().map_err(|_| {
        super::super::super::support::unsupported(span, "array capacity is not concrete")
    })?;
    if capacity == 0 {
        return Err(super::super::super::support::unsupported(
            span,
            "fixed-array capacity must be non-zero",
        ));
    }
    Ok(capacity)
}
