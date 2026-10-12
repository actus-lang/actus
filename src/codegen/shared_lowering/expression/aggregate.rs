// Aggregate helpers return the complete neutral location contract.
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

use crate::ast::{CompoundAssignmentOp, Expr, StructFieldInit, TypeName};
use crate::lexer::SourceSpan;

use super::super::super::{
    BackendAggregateField, BackendContractError, BackendExpressionProvider, BackendInstruction,
    BackendIntegerBinaryOp, BackendValueId, BackendValueType,
};
use super::super::SharedExpressionProvider;
use super::super::support::expression_span;
use super::super::support::unsupported;
use super::packed;

pub(super) fn parameter_nominal_types(
    verb: &crate::ast::VerbDecl,
    layouts: &dyn super::super::super::BackendLayoutProvider,
) -> std::collections::BTreeMap<BackendValueId, TypeName> {
    verb.params
        .iter()
        .enumerate()
        .filter_map(|(index, parameter)| {
            layouts
                .layout_for(&parameter.ty)
                .map(|_| (BackendValueId(index as u32), parameter.ty.clone()))
        })
        .collect()
}

pub(super) fn lower_aggregate_expression(
    provider: &mut SharedExpressionProvider<'_>,
    expression: &Expr,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    match expression {
        Expr::StructLit { .. } => {
            lower_struct_literal_expression(provider, expression, instructions)
        }
        Expr::FieldAccess { object, field, span } => {
            lower_field_access_expression(provider, object, field, *span, instructions)
        }
        Expr::Index { target, index, span } => {
            lower_array_element(provider, target, index, *span, instructions)
        }
        _ => Err(unsupported(expression_span(expression), "aggregate expression expected")),
    }
}

pub(super) fn lower_array_element(
    provider: &mut SharedExpressionProvider<'_>,
    collection: &Expr,
    index: &Expr,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let (base, index, aggregate_layout, element_layout, element_type_name, element_type, capacity) =
        lower_array_element_location(provider, collection, index, span, instructions)?;
    let result = provider.allocate();
    provider.remember_value(result, element_type);
    provider.remember_nominal_type(result, element_type_name);
    instructions.push(BackendInstruction::ArrayElementLoad {
        result,
        base,
        index,
        aggregate_layout,
        element_layout,
        element_type,
        capacity,
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
    let (base, index, aggregate_layout, element_layout, _, element_type, capacity) =
        lower_array_element_location(provider, collection, index, span, instructions)?;
    let value = provider.lower_expression(value, instructions)?;
    if provider.value_type(value) != Some(element_type) {
        return Err(unsupported(span, "array element assignment type mismatch"));
    }
    instructions.push(BackendInstruction::ArrayElementStore {
        base,
        index,
        aggregate_layout,
        element_layout,
        element_type,
        value,
        capacity,
        span,
    });
    Ok(())
}

pub(super) fn lower_array_element_compound_assignment(
    provider: &mut SharedExpressionProvider<'_>,
    collection: &Expr,
    index: &Expr,
    operator: CompoundAssignmentOp,
    value: &Expr,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(), BackendContractError> {
    let (base, index, aggregate_layout, element_layout, _, element_type, capacity) =
        lower_array_element_location(provider, collection, index, span, instructions)?;
    let current = provider.allocate();
    provider.remember_value(current, element_type);
    instructions.push(BackendInstruction::ArrayElementLoad {
        result: current,
        base,
        index,
        aggregate_layout,
        element_layout,
        element_type,
        capacity,
        span,
    });
    let value = provider.lower_expression(value, instructions)?;
    if provider.value_type(value) != Some(element_type) {
        return Err(unsupported(span, "array element compound assignment type mismatch"));
    }
    let result = provider.lower_integer_binary(
        compound_operation(operator),
        current,
        value,
        span,
        instructions,
    )?;
    instructions.push(BackendInstruction::ArrayElementStore {
        base,
        index,
        aggregate_layout,
        element_layout,
        element_type,
        value: result,
        capacity,
        span,
    });
    Ok(())
}

fn lower_array_element_location(
    provider: &mut SharedExpressionProvider<'_>,
    collection: &Expr,
    index: &Expr,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<
    (
        BackendValueId,
        BackendValueId,
        super::super::super::BackendLayout,
        super::super::super::BackendLayout,
        TypeName,
        BackendValueType,
        u32,
    ),
    BackendContractError,
> {
    let base = provider.lower_expression(collection, instructions)?;
    let index = provider.lower_expression(index, instructions)?;
    let type_name = provider
        .nominal_type(base)
        .cloned()
        .ok_or_else(|| unsupported(span, "array base has no nominal type"))?;
    if type_name.name != "Array" || type_name.arguments.len() != 2 {
        return Err(unsupported(span, "array element access requires a fixed array"));
    }
    let element_type_name = type_name.arguments[0].clone();
    let capacity = type_name.arguments[1]
        .name
        .parse::<u32>()
        .map_err(|_| unsupported(span, "array capacity is not a concrete integer"))?;
    let aggregate_layout = provider
        .layouts
        .layout_for(&type_name)
        .ok_or_else(|| unsupported(span, "array base has no neutral layout"))?;
    let element_layout = provider
        .layouts
        .layout_for(&element_type_name)
        .ok_or_else(|| unsupported(span, "array element has no neutral layout"))?;
    let element_type =
        super::super::support::backend_value_type(provider.layouts, &element_type_name)
            .ok_or_else(|| unsupported(span, "array element has no backend type"))?;
    if element_layout.size == 0
        || u64::from(element_layout.size) * u64::from(capacity) != u64::from(aggregate_layout.size)
    {
        return Err(unsupported(span, "array element layout does not match capacity"));
    }
    if !matches!(provider.value_type(index), Some(BackendValueType::Integer { .. })) {
        return Err(unsupported(span, "array index must be an integer"));
    }
    instructions.push(BackendInstruction::TrapIfArrayIndexOutOfBounds {
        index,
        capacity,
        reason: "fixed-array index is outside the declared capacity".to_owned(),
        span,
    });
    Ok((base, index, aggregate_layout, element_layout, element_type_name, element_type, capacity))
}

pub(super) fn lower_struct_literal(
    provider: &mut SharedExpressionProvider<'_>,
    name: &str,
    type_arguments: &[TypeName],
    fields: &[StructFieldInit],
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let type_name = TypeName {
        name: name.to_owned(),
        arguments: type_arguments.to_vec(),
        reference_role: None,
        span,
    };
    let layout = provider
        .layouts
        .layout_for(&type_name)
        .ok_or_else(|| unsupported(span, format!("aggregate `{name}` has no neutral layout")))?;
    let mut lowered_fields = Vec::with_capacity(fields.len());
    for field in fields {
        let metadata = provider.layouts.field_for(&type_name, &field.name).ok_or_else(|| {
            unsupported(
                field.span,
                format!("aggregate field `{}` has no neutral layout", field.name),
            )
        })?;
        let value = provider.lower_expression(&field.value, instructions)?;
        let ty = provider
            .value_type(value)
            .ok_or_else(|| unsupported(field.span, "aggregate field value type is unavailable"))?;
        lowered_fields.push(BackendAggregateField {
            offset: metadata.offset,
            layout: metadata.layout,
            value,
            ty,
        });
    }
    let result = provider.allocate();
    provider.remember_value(result, BackendValueType::Aggregate { layout });
    provider.remember_nominal_type(result, type_name);
    instructions.push(BackendInstruction::AggregateConstruct {
        result,
        layout,
        fields: lowered_fields,
        span,
    });
    Ok(result)
}

pub(super) fn lower_field_access_expression(
    provider: &mut SharedExpressionProvider<'_>,
    object: &Expr,
    field: &str,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let base = provider.lower_expression(object, instructions)?;
    let type_name = provider
        .nominal_type(base)
        .cloned()
        .ok_or_else(|| unsupported(span, "aggregate field access has no nominal type"))?;
    let aggregate_layout = provider
        .layouts
        .layout_for(&type_name)
        .ok_or_else(|| unsupported(span, "aggregate base has no neutral layout"))?;
    if provider.layouts.packed_field_for(&type_name, field).is_some() {
        return packed::lower_field_access(
            provider,
            base,
            &type_name,
            field,
            aggregate_layout,
            span,
            instructions,
        );
    }
    let metadata = provider.layouts.field_for(&type_name, field).ok_or_else(|| {
        unsupported(span, format!("aggregate field `{field}` has no neutral layout"))
    })?;
    let ty = super::super::support::backend_value_type(provider.layouts, &metadata.ty)
        .ok_or_else(|| unsupported(span, "aggregate field type has no neutral layout"))?;
    let result = provider.allocate();
    provider.remember_value(result, ty);
    instructions.push(BackendInstruction::AggregateFieldLoad {
        result,
        base,
        aggregate_layout,
        offset: metadata.offset,
        field_layout: metadata.layout,
        ty,
        span,
    });
    Ok(result)
}

pub(super) fn lower_field_assignment(
    provider: &mut SharedExpressionProvider<'_>,
    object: &Expr,
    field: &str,
    value: &Expr,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(), BackendContractError> {
    let base = provider.lower_expression(object, instructions)?;
    let type_name = provider
        .nominal_type(base)
        .cloned()
        .ok_or_else(|| unsupported(span, "aggregate field assignment has no nominal type"))?;
    if provider.layouts.packed_field_for(&type_name, field).is_some() {
        return packed::lower_field_assignment(
            provider,
            base,
            &type_name,
            field,
            value,
            span,
            instructions,
        );
    }
    let metadata = provider.layouts.field_for(&type_name, field).ok_or_else(|| {
        unsupported(span, format!("aggregate field `{field}` has no neutral layout"))
    })?;
    let value = provider.lower_expression(value, instructions)?;
    let ty = super::super::support::backend_value_type(provider.layouts, &metadata.ty)
        .ok_or_else(|| unsupported(span, "aggregate field type has no neutral layout"))?;
    let value_ty = provider
        .value_type(value)
        .ok_or_else(|| unsupported(span, "aggregate field value type is unavailable"))?;
    if value_ty != ty {
        return Err(unsupported(span, "aggregate field assignment type mismatch"));
    }
    instructions.push(BackendInstruction::AggregateFieldStore {
        base,
        aggregate_layout: provider
            .layouts
            .layout_for(&type_name)
            .ok_or_else(|| unsupported(span, "aggregate base has no neutral layout"))?,
        offset: metadata.offset,
        field_layout: metadata.layout,
        value,
        ty,
        span,
    });
    Ok(())
}

pub(super) fn lower_field_compound_assignment(
    provider: &mut SharedExpressionProvider<'_>,
    object: &Expr,
    field: &str,
    operator: CompoundAssignmentOp,
    value: &Expr,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(), BackendContractError> {
    let base = provider.lower_expression(object, instructions)?;
    let type_name = provider.nominal_type(base).cloned().ok_or_else(|| {
        unsupported(span, "aggregate field compound assignment has no nominal type")
    })?;
    let aggregate_layout = provider
        .layouts
        .layout_for(&type_name)
        .ok_or_else(|| unsupported(span, "aggregate base has no neutral layout"))?;
    if provider.layouts.packed_field_for(&type_name, field).is_some() {
        return packed::lower_field_compound_assignment(
            provider,
            base,
            &type_name,
            field,
            operator,
            value,
            aggregate_layout,
            span,
            instructions,
        );
    }
    lower_plain_field_compound_assignment(
        provider,
        base,
        &type_name,
        field,
        operator,
        value,
        aggregate_layout,
        span,
        instructions,
    )
}

fn lower_plain_field_compound_assignment(
    provider: &mut SharedExpressionProvider<'_>,
    base: BackendValueId,
    type_name: &TypeName,
    field: &str,
    operator: CompoundAssignmentOp,
    value: &Expr,
    aggregate_layout: super::super::super::BackendLayout,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(), BackendContractError> {
    let metadata = provider.layouts.field_for(type_name, field).ok_or_else(|| {
        unsupported(span, format!("aggregate field `{field}` has no neutral layout"))
    })?;
    let ty = super::super::support::backend_value_type(provider.layouts, &metadata.ty)
        .ok_or_else(|| unsupported(span, "aggregate field type has no neutral layout"))?;
    let current = provider.allocate();
    provider.remember_value(current, ty);
    instructions.push(BackendInstruction::AggregateFieldLoad {
        result: current,
        base,
        aggregate_layout,
        offset: metadata.offset,
        field_layout: metadata.layout,
        ty,
        span,
    });
    let value = provider.lower_expression(value, instructions)?;
    let value_ty = provider
        .value_type(value)
        .ok_or_else(|| unsupported(span, "aggregate field value type is unavailable"))?;
    if value_ty != ty {
        return Err(unsupported(span, "aggregate field compound assignment type mismatch"));
    }
    let operation = compound_operation(operator);
    let result = provider.lower_integer_binary(operation, current, value, span, instructions)?;
    instructions.push(BackendInstruction::AggregateFieldStore {
        base,
        aggregate_layout,
        offset: metadata.offset,
        field_layout: metadata.layout,
        value: result,
        ty,
        span,
    });
    Ok(())
}

fn compound_operation(operator: CompoundAssignmentOp) -> BackendIntegerBinaryOp {
    match operator {
        CompoundAssignmentOp::Add => BackendIntegerBinaryOp::Add,
        CompoundAssignmentOp::Subtract => BackendIntegerBinaryOp::Subtract,
        CompoundAssignmentOp::Multiply => BackendIntegerBinaryOp::Multiply,
        CompoundAssignmentOp::Divide => BackendIntegerBinaryOp::Divide,
        CompoundAssignmentOp::Remainder => BackendIntegerBinaryOp::Remainder,
        CompoundAssignmentOp::ShiftLeft => BackendIntegerBinaryOp::ShiftLeft,
        CompoundAssignmentOp::ShiftRight => BackendIntegerBinaryOp::ShiftRight,
        CompoundAssignmentOp::BitwiseAnd => BackendIntegerBinaryOp::BitwiseAnd,
        CompoundAssignmentOp::BitwiseOr => BackendIntegerBinaryOp::BitwiseOr,
        CompoundAssignmentOp::BitwiseXor => BackendIntegerBinaryOp::BitwiseXor,
    }
}

pub(super) fn lower_struct_literal_expression(
    provider: &mut SharedExpressionProvider<'_>,
    expression: &Expr,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let Expr::StructLit { name, type_arguments, fields, span } = expression else {
        return Err(unsupported(expression_span(expression), "struct literal expected"));
    };
    lower_struct_literal(provider, name, type_arguments, fields, *span, instructions)
}
