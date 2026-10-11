use crate::ast::{Expr, StructFieldInit, TypeName};
use crate::lexer::SourceSpan;

use super::super::super::{
    BackendAggregateField, BackendContractError, BackendExpressionProvider, BackendInstruction,
    BackendValueId, BackendValueType,
};
use super::super::SharedExpressionProvider;
use super::super::support::expression_span;
use super::super::support::unsupported;

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
        _ => Err(unsupported(expression_span(expression), "aggregate expression expected")),
    }
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
        .ok_or_else(|| unsupported(span, "aggregate field access has no nominal type"))?;
    let aggregate_layout = provider
        .layouts
        .layout_for(type_name)
        .ok_or_else(|| unsupported(span, "aggregate base has no neutral layout"))?;
    let metadata = provider.layouts.field_for(type_name, field).ok_or_else(|| {
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
