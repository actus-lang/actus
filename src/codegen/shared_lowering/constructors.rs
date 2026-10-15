use crate::ast::{Expr, TypeName};
use crate::lexer::SourceSpan;

use super::super::{
    BackendContractError, BackendEnumLayout, BackendExpressionProvider, BackendInstruction,
    BackendIntegerCompareOp, BackendValueId, BackendValueType,
};
use super::expression::SharedExpressionProvider;
use super::support::{expression_span, unsupported};

pub(super) fn lower_method_call(
    provider: &mut SharedExpressionProvider<'_>,
    expression: &Expr,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let Expr::MethodCall { receiver, method, arguments, span } = expression else {
        return Err(unsupported(expression_span(expression), "method call expected"));
    };
    let Expr::Identifier { name, .. } = receiver.as_ref() else {
        return Err(unsupported(*span, "Result constructor receiver must be a type name"));
    };
    let result_type = crate::semantic::parse_type_name_key(name, *span)
        .or_else(|| parse_type_name(name, *span))
        .ok_or_else(|| {
            unsupported(*span, "Result constructor receiver has an invalid type name")
        })?;
    if result_type.name == "Option" && result_type.arguments.len() == 1 {
        let layout = provider
            .enum_layout(&result_type)
            .ok_or_else(|| unsupported(*span, "Option has no neutral layout"))?;
        return if layout.niche_pointer {
            lower_niche_option_constructor(
                provider,
                &result_type,
                method,
                arguments,
                *span,
                instructions,
            )
        } else {
            lower_regular_option_constructor(
                provider,
                &result_type,
                layout,
                method,
                arguments,
                *span,
                instructions,
            )
        };
    }
    if result_type.name != "Result" || result_type.arguments.len() != 2 {
        return lower_named_unit_enum(
            provider,
            &result_type,
            method,
            arguments,
            *span,
            instructions,
        );
    }
    lower_result_constructor(provider, &result_type, method, arguments, *span, instructions)
}

fn lower_regular_option_constructor(
    provider: &mut SharedExpressionProvider<'_>,
    option_type: &TypeName,
    layout: BackendEnumLayout,
    method: &str,
    arguments: &[crate::ast::Argument],
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let (discriminant, payload) = match method {
        "None" if arguments.is_empty() => (1, None),
        "Some" if arguments.len() == 1 => {
            (0, Some(provider.lower_expression(&arguments[0].expression, instructions)?))
        }
        _ => return Err(unsupported(span, "invalid Option constructor")),
    };
    let result = provider.allocate();
    let result_type = BackendValueType::Enum { layout };
    provider.remember_value(result, result_type);
    provider.remember_nominal_type(result, option_type.clone());
    let payload_type = payload
        .map(|value| {
            provider
                .value_type(value)
                .ok_or_else(|| unsupported(span, "Option payload type is unavailable"))
        })
        .transpose()?;
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

fn lower_niche_option_constructor(
    provider: &mut SharedExpressionProvider<'_>,
    option_type: &TypeName,
    method: &str,
    arguments: &[crate::ast::Argument],
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let layout = provider
        .enum_layout(option_type)
        .filter(|layout| layout.niche_pointer)
        .ok_or_else(|| unsupported(span, "Option payload is not a pointer reference"))?;
    let (discriminant, payload) = match method {
        "None" if arguments.is_empty() => (0, None),
        "Some" if arguments.len() == 1 => {
            let payload = provider.lower_expression(&arguments[0].expression, instructions)?;
            (1, Some(payload))
        }
        _ => return Err(unsupported(span, "invalid pointer Option constructor")),
    };
    let result = provider.allocate();
    provider.remember_value(
        result,
        BackendValueType::NichePointerOption { layout: layout.layout, payload_layout: None },
    );
    provider.remember_nominal_type(result, option_type.clone());
    if let Some(payload) = payload {
        let payload_type = provider
            .value_type(payload)
            .ok_or_else(|| unsupported(span, "Option payload type is unavailable"))?;
        provider.remember_enum_payload(result, 1, payload_type);
        instructions.push(BackendInstruction::EnumConstruct {
            result,
            discriminant,
            payload: Some(payload),
            layout,
            payload_type: Some(payload_type),
            span,
        });
    } else {
        instructions.push(BackendInstruction::EnumConstruct {
            result,
            discriminant,
            payload: None,
            layout,
            payload_type: None,
            span,
        });
    }
    Ok(result)
}

fn lower_result_constructor(
    provider: &mut SharedExpressionProvider<'_>,
    result_type: &TypeName,
    method: &str,
    arguments: &[crate::ast::Argument],
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let discriminant = match method {
        "Ok" => 0,
        "Err" => 1,
        _ => return Err(unsupported(span, "unknown Result constructor")),
    };
    let [argument] = arguments else {
        return Err(unsupported(span, "Result constructors require exactly one payload"));
    };
    let unit_payload = matches!(
        &argument.expression,
        Expr::Call { callee, .. }
            if provider.call_returns_unit(callee)
    );
    if unit_payload {
        super::expression::lower_unit_call(provider, &argument.expression, instructions)?;
        return construct_result_unit(provider, result_type, discriminant, span, instructions);
    }
    let payload = provider.lower_expression(&argument.expression, instructions)?;
    let payload_type = provider
        .value_type(payload)
        .ok_or_else(|| unsupported(span, "Result payload type is unavailable"))?;
    lower_result_constructor_payload(
        provider,
        result_type,
        method,
        payload,
        payload_type,
        span,
        instructions,
    )
}

pub(super) fn lower_result_constructor_payload(
    provider: &mut SharedExpressionProvider<'_>,
    result_type: &TypeName,
    method: &str,
    payload: BackendValueId,
    payload_type: BackendValueType,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let discriminant = match method {
        "Ok" => 0,
        "Err" => 1,
        _ => return Err(unsupported(span, "unknown Result constructor")),
    };
    let layout = provider
        .enum_layout(result_type)
        .ok_or_else(|| unsupported(span, "Result payload layout is unavailable"))?;
    let result = provider.allocate();
    provider.remember_value(result, BackendValueType::Enum { layout });
    provider.remember_nominal_type(result, result_type.clone());
    provider.remember_enum_payload(result, discriminant, payload_type);
    instructions.push(BackendInstruction::EnumConstruct {
        result,
        discriminant,
        payload: Some(payload),
        layout,
        payload_type: Some(payload_type),
        span,
    });
    Ok(result)
}

fn construct_result_unit(
    provider: &mut SharedExpressionProvider<'_>,
    result_type: &TypeName,
    discriminant: u32,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let layout = provider
        .enum_layout(result_type)
        .ok_or_else(|| unsupported(span, "Result unit payload has no neutral layout"))?;
    let result = provider.allocate();
    provider.remember_value(result, BackendValueType::Enum { layout });
    provider.remember_nominal_type(result, result_type.clone());
    instructions.push(BackendInstruction::EnumConstruct {
        result,
        discriminant,
        payload: None,
        layout,
        payload_type: None,
        span,
    });
    Ok(result)
}

pub(super) fn lower_named_unit_enum(
    provider: &mut SharedExpressionProvider<'_>,
    type_name: &TypeName,
    variant: &str,
    arguments: &[crate::ast::Argument],
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let Some((discriminant, is_unit)) = provider.enum_variant(type_name, variant) else {
        return Err(unsupported(span, "unknown named enum variant"));
    };
    let layout = provider
        .enum_layout(type_name)
        .ok_or_else(|| unsupported(span, "named enum has no neutral layout"))?;
    if layout.niche_pointer {
        return lower_niche_option_constructor(
            provider,
            type_name,
            variant,
            arguments,
            span,
            instructions,
        );
    }
    lower_named_enum_constructor(
        provider,
        type_name,
        discriminant,
        is_unit,
        layout,
        arguments,
        span,
        instructions,
    )
}

fn lower_named_enum_constructor(
    provider: &mut SharedExpressionProvider<'_>,
    type_name: &TypeName,
    discriminant: u32,
    is_unit: bool,
    layout: BackendEnumLayout,
    arguments: &[crate::ast::Argument],
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let payload = if is_unit {
        if !arguments.is_empty() {
            return Err(unsupported(span, "named enum unit variants cannot carry a payload"));
        }
        None
    } else {
        let [argument] = arguments else {
            return Err(unsupported(span, "named enum constructors require exactly one payload"));
        };
        Some(provider.lower_expression(&argument.expression, instructions)?)
    };
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

pub(super) fn lower_enum_variant_condition_value(
    provider: &mut SharedExpressionProvider<'_>,
    subject_value: BackendValueId,
    variant: &str,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let subject_type = provider
        .value_type(subject_value)
        .ok_or_else(|| unsupported(span, "enum case subject is not a neutral enum value"))?;
    let layout = if matches!(subject_type, BackendValueType::Integer { .. }) {
        provider
            .nominal_type(subject_value)
            .and_then(|type_name| provider.enum_layout(type_name))
            .ok_or_else(|| unsupported(span, "unit enum case has no neutral layout"))?
    } else {
        enum_case_layout(subject_type, span)?
    };
    let discriminant = enum_case_discriminant(provider, layout, subject_type, variant, span)?;
    let integer = BackendValueType::Integer { signed: false, width: 32 };
    let actual = if matches!(subject_type, BackendValueType::Integer { .. }) {
        subject_value
    } else {
        let actual = provider.allocate();
        provider.remember_value(actual, integer);
        instructions.push(BackendInstruction::EnumDiscriminant {
            result: actual,
            subject: subject_value,
            layout,
            span,
        });
        actual
    };
    let expected = provider.allocate();
    provider.remember_value(expected, integer);
    instructions.push(BackendInstruction::Constant {
        result: expected,
        ty: integer,
        value: u64::from(discriminant),
        span,
    });
    let condition = provider.allocate();
    provider.remember_value(condition, BackendValueType::Bool);
    instructions.push(BackendInstruction::IntegerCompare {
        result: condition,
        operation: BackendIntegerCompareOp::Equals,
        left: actual,
        right: expected,
        signed: false,
        span,
    });
    Ok(condition)
}

fn enum_case_layout(
    subject_type: BackendValueType,
    span: SourceSpan,
) -> Result<BackendEnumLayout, BackendContractError> {
    match subject_type {
        BackendValueType::Enum { layout } => Ok(layout),
        BackendValueType::NichePointerOption { layout, .. } => Ok(BackendEnumLayout {
            layout,
            discriminant_offset: 0,
            discriminant_size: 4,
            payload_offset: 0,
            payload: None,
            niche_pointer: true,
        }),
        _ => Err(unsupported(span, "enum case subject is not a neutral enum value")),
    }
}

fn enum_case_discriminant(
    provider: &SharedExpressionProvider<'_>,
    layout: BackendEnumLayout,
    subject_type: BackendValueType,
    variant: &str,
    span: SourceSpan,
) -> Result<u32, BackendContractError> {
    let niche_option = matches!(subject_type, BackendValueType::NichePointerOption { .. });
    match variant {
        "Ok" => Ok(0),
        "Err" => Ok(1),
        "Some" if niche_option => Ok(1),
        "None" if niche_option => Ok(0),
        "Some" => Ok(0),
        "None" => Ok(1),
        _ => provider
            .enum_variant_for_layout(layout, variant)
            .map(|(id, _)| id)
            .ok_or_else(|| unsupported(span, "unknown enum case variant")),
    }
}

fn parse_type_name(input: &str, span: SourceSpan) -> Option<TypeName> {
    let Some(open) = input.find('[') else {
        return Some(TypeName {
            name: input.to_owned(),
            arguments: Vec::new(),
            reference_role: None,
            span,
        });
    };
    if !input.ends_with(']') {
        return None;
    }
    let arguments = input[open + 1..input.len() - 1]
        .split(',')
        .map(|argument| parse_type_name(argument.trim(), span))
        .collect::<Option<Vec<_>>>()?;
    Some(TypeName { name: input[..open].to_owned(), arguments, reference_role: None, span })
}
