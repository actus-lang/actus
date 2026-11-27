use crate::ast::{Expr, TypeName};
use crate::codegen::{
    BackendContractError, BackendExpressionProvider, BackendInstruction, BackendValueId,
    BackendValueType,
};
use crate::lexer::SourceSpan;

use super::super::super::SharedExpressionProvider;
use super::super::super::errors::unsupported;

pub(super) fn lower_field_access_expression(
    provider: &mut SharedExpressionProvider<'_>,
    object: &Expr,
    field: &str,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let base = provider.lower_expression(object, instructions)?;
    let type_name = field_type_name(provider, base, span)?;
    let aggregate_layout = provider
        .layouts
        .layout_for(&type_name)
        .ok_or_else(|| unsupported(span, "aggregate base has no neutral layout"))?;
    if provider
        .layouts
        .packed_field_for(&type_name, field)
        .is_some_and(|metadata| metadata.ty.name != "Array")
    {
        return super::super::packed::lower_field_access(
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
    let declared_ty =
        super::super::super::scalar_types::backend_value_type(provider.layouts, &metadata.ty)
            .ok_or_else(|| unsupported(span, "aggregate field type has no neutral layout"))?;
    let ty = indirect_field_type(metadata.indirect, declared_ty);
    let result = provider.allocate();
    provider.remember_value(result, ty);
    remember_field_metadata(provider, result, &metadata.ty, &type_name, field);
    if matches!(ty, BackendValueType::Aggregate { .. }) {
        provider.remember_aggregate_place(result);
    }
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
    if provider
        .layouts
        .packed_field_for(&type_name, field)
        .is_some_and(|metadata| metadata.ty.name != "Array")
    {
        return super::super::packed::lower_field_assignment(
            provider,
            base,
            &type_name,
            field,
            value,
            span,
            instructions,
        );
    }
    let value = provider.lower_expression(value, instructions)?;
    store_field_value(provider, base, &type_name, field, value, span, instructions)
}

/// Assigns a struct literal to the whole owner `name` that holds a resource:
/// every field value is evaluated first, then the value the owner held is
/// released, then the new fields are stored (ADR-0125), so a field value
/// that reads the old value (`p = Pair { a: f(t: abs p.a), ... }`) reads it
/// before it is released, as any other assignment does.
pub(super) fn assign_struct_literal_owner(
    provider: &mut SharedExpressionProvider<'_>,
    name: &str,
    fields: &[crate::ast::StructFieldInit],
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(), BackendContractError> {
    let mut values = Vec::with_capacity(fields.len());
    for field in fields {
        values.push(provider.lower_expression(&field.value, instructions)?);
    }
    super::super::cleanup::emit_replacement(provider, span, instructions);
    let base = provider
        .lower_expression(&Expr::Identifier { name: name.to_owned(), span }, instructions)?;
    let type_name = provider
        .nominal_type(base)
        .cloned()
        .ok_or_else(|| unsupported(span, "aggregate assignment has no nominal type"))?;
    for (field, value) in fields.iter().zip(values) {
        store_field_value(
            provider,
            base,
            &type_name,
            &field.name,
            value,
            field.span,
            instructions,
        )?;
    }
    Ok(())
}

/// Stores the lowered `value` into the field `field` of `base`, a value of
/// `type_name`. The value the field held is released first when the
/// analyzer planned it for the assignment at `span` (ADR-0125): the new value
/// was evaluated already.
fn store_field_value(
    provider: &mut SharedExpressionProvider<'_>,
    base: BackendValueId,
    type_name: &TypeName,
    field: &str,
    value: BackendValueId,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(), BackendContractError> {
    let metadata = provider.layouts.field_for(type_name, field).ok_or_else(|| {
        unsupported(span, format!("aggregate field `{field}` has no neutral layout"))
    })?;
    let declared_ty =
        super::super::super::scalar_types::backend_value_type(provider.layouts, &metadata.ty)
            .ok_or_else(|| unsupported(span, "aggregate field type has no neutral layout"))?;
    let ty = indirect_field_type(metadata.indirect, declared_ty);
    let value =
        super::super::state::coerce_value_to_type(provider, value, Some(ty), span, instructions)?;
    if provider.value_type(value) != Some(ty) {
        return Err(unsupported(span, "aggregate field assignment type mismatch"));
    }
    let aggregate_layout = provider
        .layouts
        .layout_for(type_name)
        .ok_or_else(|| unsupported(span, "aggregate base has no neutral layout"))?;
    super::super::cleanup::emit_place_replacement(provider, span, base, instructions);
    instructions.push(BackendInstruction::AggregateFieldStore {
        base,
        aggregate_layout,
        offset: metadata.offset,
        field_layout: metadata.layout,
        value,
        ty,
        span,
    });
    if matches!(ty, BackendValueType::Aggregate { .. }) {
        super::super::cleanup::release_moved_storage(provider, value, span, instructions);
    }
    Ok(())
}

fn field_type_name(
    provider: &SharedExpressionProvider<'_>,
    base: BackendValueId,
    span: SourceSpan,
) -> Result<TypeName, BackendContractError> {
    provider
        .nominal_type(base)
        .cloned()
        .or_else(|| {
            provider.value_type(base).and_then(|value_type| match value_type {
                BackendValueType::Aggregate { layout } => provider.layouts.type_for_layout(layout),
                _ => None,
            })
        })
        .ok_or_else(|| {
            unsupported(
                span,
                format!(
                    "aggregate field access has no nominal type (value type: {:?})",
                    provider.value_type(base)
                ),
            )
        })
}

fn indirect_field_type(indirect: bool, declared_ty: BackendValueType) -> BackendValueType {
    if indirect && !matches!(declared_ty, BackendValueType::NichePointerOption { .. }) {
        BackendValueType::Pointer
    } else {
        declared_ty
    }
}

fn remember_field_metadata(
    provider: &mut SharedExpressionProvider<'_>,
    result: BackendValueId,
    field_type: &TypeName,
    aggregate_type: &TypeName,
    field: &str,
) {
    if provider.layouts.layout_for(field_type).is_some() {
        provider.remember_nominal_type(result, field_type.clone());
        if let Some(packed) = provider.layouts.packed_field_for(aggregate_type, field) {
            provider.remember_array_endianness(result, packed.big_endian);
        }
    }
}
