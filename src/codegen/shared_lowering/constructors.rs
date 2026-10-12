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
    let result_type = parse_type_name(name, *span).ok_or_else(|| {
        unsupported(*span, "Result constructor receiver has an invalid type name")
    })?;
    if result_type.name == "Option" && result_type.arguments.len() == 1 {
        return lower_niche_option_constructor(
            provider,
            &result_type,
            method,
            arguments,
            *span,
            instructions,
        );
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
    provider.remember_value(result, BackendValueType::NichePointerOption { layout: layout.layout });
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
    let layout = provider
        .enum_layout(result_type)
        .ok_or_else(|| unsupported(span, "Result payload layout is unavailable"))?;
    let result = provider.allocate();
    provider.remember_value(result, BackendValueType::Enum { layout });
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
    if !arguments.is_empty() {
        return Err(unsupported(span, "named enum unit variants cannot carry a payload"));
    }
    let Some((discriminant, is_unit)) = provider.enum_variant(type_name, variant) else {
        return Err(unsupported(span, "unknown named enum variant"));
    };
    if !is_unit {
        return Err(unsupported(span, "named enum payload constructors are not lowered"));
    }
    let layout = provider
        .enum_layout(type_name)
        .ok_or_else(|| unsupported(span, "named enum has no neutral layout"))?;
    let result = provider.allocate();
    provider.remember_value(result, BackendValueType::Enum { layout });
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

pub(super) fn lower_enum_variant_condition_value(
    provider: &mut SharedExpressionProvider<'_>,
    subject_value: BackendValueId,
    variant: &str,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let Some(subject_type) = provider.value_type(subject_value) else {
        return Err(unsupported(span, "enum case subject is not a neutral enum value"));
    };
    let layout = match subject_type {
        BackendValueType::Enum { layout } => layout,
        BackendValueType::NichePointerOption { layout } => BackendEnumLayout {
            layout,
            discriminant_offset: 0,
            discriminant_size: 4,
            payload_offset: 0,
            payload: None,
            niche_pointer: true,
        },
        _ => return Err(unsupported(span, "enum case subject is not a neutral enum value")),
    };
    let discriminant = match variant {
        "Ok" => 0,
        "Err" => 1,
        _ => provider
            .enum_variant_for_layout(layout, variant)
            .map(|(id, _)| id)
            .ok_or_else(|| unsupported(span, "unknown enum case variant"))?,
    };
    let actual = provider.allocate();
    let integer = BackendValueType::Integer { signed: false, width: 32 };
    provider.remember_value(actual, integer);
    instructions.push(BackendInstruction::EnumDiscriminant {
        result: actual,
        subject: subject_value,
        layout,
        span,
    });
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
