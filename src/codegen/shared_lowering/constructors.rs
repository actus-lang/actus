use crate::ast::{Expr, TypeName};
use crate::lexer::SourceSpan;

use super::super::{
    BackendContractError, BackendExpressionProvider, BackendInstruction, BackendIntegerCompareOp,
    BackendLayout, BackendValueId, BackendValueType,
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
    let discriminant = match method.as_str() {
        "Ok" => 0,
        "Err" => 1,
        _ => return Err(unsupported(*span, "unknown Result constructor")),
    };
    if arguments.len() != 1 {
        return Err(unsupported(*span, "Result constructors require exactly one payload"));
    }
    let payload = provider.lower_expression(&arguments[0].expression, instructions)?;
    let payload_type = provider
        .value_type(payload)
        .ok_or_else(|| unsupported(*span, "Result payload type is unavailable"))?;
    let layout = result_layout(payload_type)
        .ok_or_else(|| unsupported(*span, "Result payload layout is unavailable"))?;
    let result = provider.allocate();
    provider.remember_value(result, BackendValueType::Enum { layout });
    provider.remember_enum_payload(result, discriminant, payload_type);
    instructions.push(BackendInstruction::EnumConstruct {
        result,
        discriminant,
        payload: Some(payload),
        layout,
        payload_type: Some(payload_type),
        span: *span,
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
    let Some(BackendValueType::Enum { layout }) = provider.value_type(subject_value) else {
        return Err(unsupported(span, "enum case subject is not a neutral enum value"));
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

fn result_layout(payload_type: BackendValueType) -> Option<super::super::BackendEnumLayout> {
    let payload = match payload_type {
        BackendValueType::Bool | BackendValueType::Integer { width: 8, .. } => {
            BackendLayout { size: 1, alignment: 1 }
        }
        BackendValueType::Integer { width: 16, .. } => BackendLayout { size: 2, alignment: 2 },
        BackendValueType::Integer { width: 32, .. } => BackendLayout { size: 4, alignment: 4 },
        BackendValueType::Integer { width: 64, .. } => BackendLayout { size: 8, alignment: 8 },
        BackendValueType::Aggregate { layout }
        | BackendValueType::Region { layout }
        | BackendValueType::Enum { layout: super::super::BackendEnumLayout { layout, .. } } => {
            layout
        }
        _ => return None,
    };
    let payload_offset = 4u32.max(payload.alignment);
    let alignment = 4u32.max(payload.alignment);
    let size = (payload_offset + payload.size).div_ceil(alignment) * alignment;
    Some(super::super::BackendEnumLayout {
        layout: BackendLayout { size, alignment },
        discriminant_offset: 0,
        discriminant_size: 4,
        payload_offset,
        payload: Some(payload),
        niche_pointer: false,
    })
}
