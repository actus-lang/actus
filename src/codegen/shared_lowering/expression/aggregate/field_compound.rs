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
            provider, base, &type_name, field, operator, value, aggregate_layout, span,
            instructions,
        );
    }
    lower_plain_field_compound_assignment(
        provider, base, &type_name, field, operator, value, aggregate_layout, span, instructions,
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
    let ty = if metadata.indirect {
        BackendValueType::Pointer
    } else {
        super::super::support::backend_value_type(provider.layouts, &metadata.ty)
            .ok_or_else(|| unsupported(span, "aggregate field type has no neutral layout"))?
    };
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
    let value = super::state::coerce_value_to_type(
        provider,
        value,
        Some(ty),
        span,
        instructions,
    )?;
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
