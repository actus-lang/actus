use crate::ast::{Expr, StructFieldInit, TypeName};
use crate::codegen::{
    BackendAggregateField, BackendContractError, BackendExpressionProvider, BackendInstruction,
    BackendValueId, BackendValueType,
};
use crate::lexer::SourceSpan;

use super::super::super::SharedExpressionProvider;
use super::super::super::support::{expression_span, unsupported};

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
    let lowered_fields = lower_fields(provider, &type_name, fields, instructions)?;
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

fn lower_fields(
    provider: &mut SharedExpressionProvider<'_>,
    type_name: &TypeName,
    fields: &[StructFieldInit],
    instructions: &mut Vec<BackendInstruction>,
) -> Result<Vec<BackendAggregateField>, BackendContractError> {
    fields.iter().map(|field| lower_field(provider, type_name, field, instructions)).collect()
}

fn lower_field(
    provider: &mut SharedExpressionProvider<'_>,
    type_name: &TypeName,
    field: &StructFieldInit,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendAggregateField, BackendContractError> {
    let metadata = provider.layouts.field_for(type_name, &field.name).ok_or_else(|| {
        unsupported(field.span, format!("aggregate field `{}` has no neutral layout", field.name))
    })?;
    let declared_ty =
        super::super::super::support::backend_value_type(provider.layouts, &metadata.ty)
            .ok_or_else(|| unsupported(field.span, "aggregate field type has no neutral layout"))?;
    let ty = indirect_field_type(metadata.indirect, declared_ty);
    let value = provider.lower_expression(&field.value, instructions)?;
    let value = super::super::state::coerce_value_to_type(
        provider,
        value,
        Some(ty),
        field.span,
        instructions,
    )?;
    Ok(BackendAggregateField { offset: metadata.offset, layout: metadata.layout, value, ty })
}

fn indirect_field_type(indirect: bool, declared_ty: BackendValueType) -> BackendValueType {
    if indirect && !matches!(declared_ty, BackendValueType::NichePointerOption { .. }) {
        BackendValueType::Pointer
    } else {
        declared_ty
    }
}
