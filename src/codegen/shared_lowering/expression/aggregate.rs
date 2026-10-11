use crate::ast::{CompoundAssignmentOp, Expr, StructFieldInit, TypeName};
use crate::lexer::SourceSpan;

use super::super::super::{
    BackendAggregateField, BackendContractError, BackendExpressionProvider, BackendInstruction,
    BackendIntegerBinaryOp, BackendValueId, BackendValueType,
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
    let metadata = provider.layouts.field_for(&type_name, field).ok_or_else(|| {
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
