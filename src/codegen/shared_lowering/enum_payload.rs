use crate::ast::TypeName;
use crate::lexer::SourceSpan;

use super::super::{
    BackendAggregateField, BackendContractError, BackendEnumLayout, BackendExpressionProvider,
    BackendInstruction, BackendValueId, BackendValueType,
};
use super::expression::SharedExpressionProvider;
use super::support::unsupported;

pub(super) fn lower_named_enum_constructor(
    provider: &mut SharedExpressionProvider<'_>,
    type_name: &TypeName,
    variant: &str,
    discriminant: u32,
    is_unit: bool,
    layout: BackendEnumLayout,
    arguments: &[crate::ast::Argument],
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let payload = lower_named_enum_payload(
        provider,
        type_name,
        variant,
        is_unit,
        layout,
        arguments,
        span,
        instructions,
    )?;
    let payload_type = payload
        .map(|value| {
            provider
                .value_type(value)
                .ok_or_else(|| unsupported(span, "named enum payload type is unavailable"))
        })
        .transpose()?;
    let result = provider.allocate();
    if layout.payload.is_none() {
        let value_type = BackendValueType::Integer { signed: false, width: 32 };
        provider.remember_value(result, value_type);
        provider.remember_nominal_type(result, type_name.clone());
        instructions.push(BackendInstruction::Constant {
            result,
            ty: value_type,
            value: u64::from(discriminant),
            span,
        });
        return Ok(result);
    }
    provider.remember_value(result, BackendValueType::Enum { layout });
    provider.remember_nominal_type(result, type_name.clone());
    if let Some(payload_type) = payload_type {
        provider.remember_enum_payload(result, discriminant, payload_type);
    }
    instructions.push(BackendInstruction::EnumConstruct {
        result,
        discriminant,
        payload,
        layout,
        payload_type,
        span,
    });
    Ok(result)
}

fn lower_named_enum_payload(
    provider: &mut SharedExpressionProvider<'_>,
    type_name: &TypeName,
    variant: &str,
    is_unit: bool,
    layout: BackendEnumLayout,
    arguments: &[crate::ast::Argument],
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<Option<BackendValueId>, BackendContractError> {
    if is_unit {
        if arguments.is_empty() {
            return Ok(None);
        }
        return Err(unsupported(span, "named enum unit variants cannot carry a payload"));
    }
    let Some(fields) = provider.enum_variant_payload_fields(type_name, variant) else {
        return Err(unsupported(span, "named enum payload fields are unavailable"));
    };
    if fields.len() <= 1 {
        let [argument] = arguments else {
            return Err(unsupported(span, "named enum constructors require exactly one payload"));
        };
        return Ok(Some(provider.lower_expression(&argument.expression, instructions)?));
    }
    let payload_layout = layout
        .payload
        .ok_or_else(|| unsupported(span, "named enum payload has no neutral layout"))?;
    let mut lowered_fields = Vec::new();
    for field in fields {
        let argument = field_argument(&field.name, arguments)
            .ok_or_else(|| unsupported(span, "named enum payload argument is unavailable"))?;
        let value = provider.lower_expression(&argument.expression, instructions)?;
        let ty = provider
            .backend_value_type(&field.ty)
            .ok_or_else(|| unsupported(span, "named enum payload field type is unavailable"))?;
        let value = provider.coerce_value_to_type(value, Some(ty), span, instructions)?;
        lowered_fields.push(BackendAggregateField {
            offset: field.offset,
            layout: field.layout,
            value,
            ty,
        });
    }
    let result = provider.allocate();
    provider.remember_value(result, BackendValueType::Aggregate { layout: payload_layout });
    instructions.push(BackendInstruction::AggregateConstruct {
        result,
        layout: payload_layout,
        fields: lowered_fields,
        span,
    });
    Ok(Some(result))
}

fn field_argument<'a>(
    field: &str,
    arguments: &'a [crate::ast::Argument],
) -> Option<&'a crate::ast::Argument> {
    field
        .parse::<usize>()
        .ok()
        .and_then(|index| arguments.get(index))
        .filter(|argument| argument.name.is_none())
        .or_else(|| arguments.iter().find(|argument| argument.name.as_deref() == Some(field)))
}
