use crate::ast::{Expr, TypeName};
use crate::codegen::{
    BackendContractError, BackendExpressionProvider, BackendInstruction, BackendLayout,
    BackendValueId, BackendValueType,
};
use crate::lexer::SourceSpan;

use super::super::super::SharedExpressionProvider;
use super::super::super::support::unsupported;

pub(super) fn lower_array_element(
    provider: &mut SharedExpressionProvider<'_>,
    collection: &Expr,
    index: &Expr,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let base = provider.lower_expression(collection, instructions)?;
    if provider.nominal_type(base).is_some_and(|type_name| type_name.name == "Buffer") {
        return super::super::buffer::lower_buffer_element_from_base(
            provider,
            base,
            index,
            span,
            instructions,
        );
    }
    let index = provider.lower_expression(index, instructions)?;
    let location =
        lower_array_element_location_from_values(provider, base, index, span, instructions)?;
    let result = provider.allocate();
    provider.remember_value(result, location.element_type);
    provider.remember_nominal_type(result, location.element_type_name);
    if matches!(location.element_type, BackendValueType::Aggregate { .. }) {
        provider.remember_aggregate_place(result);
    }
    instructions.push(BackendInstruction::ArrayElementLoad {
        result,
        base: location.base,
        index: location.index,
        aggregate_layout: location.aggregate_layout,
        element_layout: location.element_layout,
        element_type: location.element_type,
        capacity: location.capacity,
        big_endian: location.big_endian,
        span,
    });
    Ok(result)
}

pub(super) fn lower_array_element_assignment(
    provider: &mut SharedExpressionProvider<'_>,
    collection: &Expr,
    index: &Expr,
    value: &Expr,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(), BackendContractError> {
    let base_value = provider.lower_expression(collection, instructions)?;
    if provider.nominal_type(base_value).is_some_and(|type_name| type_name.name == "Buffer") {
        return super::super::buffer::lower_buffer_element_assignment_from_base(
            provider,
            base_value,
            index,
            value,
            span,
            instructions,
        );
    }
    let lowered_index = provider.lower_expression(index, instructions)?;
    let location = lower_array_element_location_from_values(
        provider,
        base_value,
        lowered_index,
        span,
        instructions,
    )?;
    let value = provider.lower_expression(value, instructions)?;
    let value = super::super::state::coerce_value_to_type(
        provider,
        value,
        Some(location.element_type),
        span,
        instructions,
    )?;
    if provider.value_type(value) != Some(location.element_type) {
        return Err(unsupported(span, "array element assignment type mismatch"));
    }
    instructions.push(BackendInstruction::ArrayElementStore {
        base: location.base,
        index: location.index,
        aggregate_layout: location.aggregate_layout,
        element_layout: location.element_layout,
        element_type: location.element_type,
        value,
        capacity: location.capacity,
        big_endian: location.big_endian,
        span,
    });
    Ok(())
}

pub(super) fn lower_array_element_location(
    provider: &mut SharedExpressionProvider<'_>,
    collection: &Expr,
    index: &Expr,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<ArrayElementLocation, BackendContractError> {
    let base = provider.lower_expression(collection, instructions)?;
    let index = provider.lower_expression(index, instructions)?;
    lower_array_element_location_from_values(provider, base, index, span, instructions)
}

pub(super) struct ArrayElementLocation {
    pub(super) base: BackendValueId,
    pub(super) index: BackendValueId,
    pub(super) aggregate_layout: BackendLayout,
    pub(super) element_layout: BackendLayout,
    pub(super) element_type_name: TypeName,
    pub(super) element_type: BackendValueType,
    pub(super) capacity: u32,
    pub(super) big_endian: bool,
}

fn lower_array_element_location_from_values(
    provider: &mut SharedExpressionProvider<'_>,
    base: BackendValueId,
    index: BackendValueId,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<ArrayElementLocation, BackendContractError> {
    let (type_name, element_type_name, capacity) = array_metadata(provider, base, span)?;
    let aggregate_layout = provider
        .layouts
        .layout_for(&type_name)
        .ok_or_else(|| unsupported(span, "array base has no neutral layout"))?;
    let element_layout = provider
        .layouts
        .layout_for(&element_type_name)
        .ok_or_else(|| unsupported(span, "array element has no neutral layout"))?;
    let element_type =
        super::super::super::support::backend_value_type(provider.layouts, &element_type_name)
            .ok_or_else(|| unsupported(span, "array element has no backend type"))?;
    validate_array_layouts(aggregate_layout, element_layout, capacity, span)?;
    if !matches!(provider.value_type(index), Some(BackendValueType::Integer { .. })) {
        return Err(unsupported(span, "array index must be an integer"));
    }
    instructions.push(BackendInstruction::TrapIfArrayIndexOutOfBounds {
        index,
        capacity,
        reason: "fixed-array index is outside the declared capacity".to_owned(),
        span,
    });
    Ok(ArrayElementLocation {
        base,
        index,
        aggregate_layout,
        element_layout,
        element_type_name,
        element_type,
        capacity,
        big_endian: provider.array_is_big_endian(base),
    })
}

fn validate_array_layouts(
    aggregate_layout: BackendLayout,
    element_layout: BackendLayout,
    capacity: u32,
    span: SourceSpan,
) -> Result<(), BackendContractError> {
    if element_layout.size == 0
        || u64::from(element_layout.size) * u64::from(capacity) != u64::from(aggregate_layout.size)
    {
        return Err(unsupported(span, "array element layout does not match capacity"));
    }
    Ok(())
}

fn array_metadata(
    provider: &SharedExpressionProvider<'_>,
    base: BackendValueId,
    span: SourceSpan,
) -> Result<(TypeName, TypeName, u32), BackendContractError> {
    let type_name = provider
        .nominal_type(base)
        .cloned()
        .ok_or_else(|| unsupported(span, "array base has no nominal type"))?;
    if type_name.name != "Array" || type_name.arguments.len() != 2 {
        return Err(unsupported(
            span,
            format!("array element access requires a fixed array (found `{}`)", type_name.name),
        ));
    }
    let capacity = type_name.arguments[1]
        .name
        .parse::<u32>()
        .map_err(|_| unsupported(span, "array capacity is not a concrete integer"))?;
    Ok((type_name.clone(), type_name.arguments[0].clone(), capacity))
}
