use super::super::super::{
    BackendContractError, BackendExpressionProvider, BackendInstruction, BackendValueId,
    BackendValueType,
};
use super::super::SharedExpressionProvider;
use super::super::support::{expression_span, unsupported};
use crate::ast::{Expr, TypeName};
use crate::lexer::SourceSpan;

pub(super) fn lower_constructor(
    provider: &mut SharedExpressionProvider<'_>,
    callee: &str,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let type_name = crate::semantic::parse_type_name_key(callee, span)
        .ok_or_else(|| unsupported(span, "malformed arena constructor"))?;
    let capacity = arena_capacity(&type_name, span)?;
    let pointer_width = pointer_width(provider, &type_name, span)?;
    let result = provider.allocate();
    provider.remember_value(result, BackendValueType::Pointer);
    provider.remember_nominal_type(result, type_name);
    instructions.push(BackendInstruction::ArenaConstruct { result, capacity, pointer_width, span });
    Ok(result)
}

pub(super) fn is_place_call(provider: &SharedExpressionProvider<'_>, expression: &Expr) -> bool {
    let Expr::MethodCall { receiver, method, .. } = expression else {
        return false;
    };
    method == "place"
        && matches!(receiver.as_ref(), Expr::Identifier { .. })
        && receiver_type(provider, receiver).is_some_and(|type_name| type_name.name == "Arena")
}

pub(super) fn lower_place(
    provider: &mut SharedExpressionProvider<'_>,
    expression: &Expr,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let Expr::MethodCall { receiver, arguments, span, .. } = expression else {
        return Err(unsupported(expression_span(expression), "arena placement expected"));
    };
    let arena_type = receiver_type(provider, receiver)
        .cloned()
        .ok_or_else(|| unsupported(*span, "arena receiver has no nominal type"))?;
    let capacity = arena_capacity(&arena_type, *span)?;
    let pointer_width = pointer_width(provider, &arena_type, *span)?;
    let arena = provider.lower_expression(receiver, instructions)?;
    let [argument] = arguments.as_slice() else {
        return Err(unsupported(*span, "arena.place requires exactly one value"));
    };
    let value = provider.lower_expression(&argument.expression, instructions)?;
    let value_type = provider
        .value_type(value)
        .ok_or_else(|| unsupported(*span, "arena placement value has no backend type"))?;
    let value_name = provider
        .nominal_type(value)
        .cloned()
        .ok_or_else(|| unsupported(*span, "arena placement value has no nominal type"))?;
    let value_layout = provider
        .layouts
        .layout_for(&value_name)
        .ok_or_else(|| unsupported(*span, "arena placement value has no neutral layout"))?;
    if !matches!(value_type, BackendValueType::Aggregate { .. }) {
        return Err(unsupported(
            *span,
            "arena.place currently requires a fixed-size aggregate value",
        ));
    }
    let result = provider.allocate();
    provider.remember_value(result, value_type);
    provider.remember_nominal_type(result, value_name);
    instructions.push(BackendInstruction::ArenaPlace {
        result,
        arena,
        value,
        capacity,
        value_layout,
        alignment: value_layout.alignment,
        pointer_width,
        span: *span,
    });
    Ok(result)
}

fn receiver_type<'a>(
    provider: &'a SharedExpressionProvider<'_>,
    receiver: &Expr,
) -> Option<&'a TypeName> {
    let Expr::Identifier { name, .. } = receiver else { return None };
    provider.binding_value(name).and_then(|value| provider.nominal_type(value))
}

fn arena_capacity(type_name: &TypeName, span: SourceSpan) -> Result<u32, BackendContractError> {
    if type_name.name != "Arena" || type_name.arguments.len() != 1 {
        return Err(unsupported(span, "arena type requires one concrete capacity"));
    }
    let capacity = type_name.arguments[0]
        .name
        .parse::<u32>()
        .map_err(|_| unsupported(span, "arena capacity is not concrete"))?;
    if capacity == 0 {
        return Err(unsupported(span, "arena capacity must be non-zero"));
    }
    Ok(capacity)
}

fn pointer_width(
    provider: &SharedExpressionProvider<'_>,
    type_name: &TypeName,
    span: SourceSpan,
) -> Result<u8, BackendContractError> {
    let layout = provider
        .layouts
        .layout_for(type_name)
        .ok_or_else(|| unsupported(span, "arena has no neutral pointer layout"))?;
    u8::try_from(layout.size)
        .ok()
        .filter(|width| matches!(width, 4 | 8))
        .ok_or_else(|| unsupported(span, "arena pointer width must be 32 or 64 bits"))
}
