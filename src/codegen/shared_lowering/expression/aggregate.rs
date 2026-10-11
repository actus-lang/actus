use crate::ast::{Expr, StructFieldInit, TypeName};
use crate::lexer::SourceSpan;

use super::super::super::{
    BackendAggregateField, BackendContractError, BackendExpressionProvider, BackendInstruction,
    BackendValueId, BackendValueType,
};
use super::super::SharedExpressionProvider;
use super::super::support::expression_span;
use super::super::support::unsupported;

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
    instructions.push(BackendInstruction::AggregateConstruct {
        result,
        layout,
        fields: lowered_fields,
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
