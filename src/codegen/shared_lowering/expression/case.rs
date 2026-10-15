use crate::ast::{CaseMode, VariantPayload};
use crate::lexer::SourceSpan;

use super::super::super::{
    BackendCaseMode, BackendContractError, BackendExpressionProvider, BackendInstruction,
    BackendValueId, BackendValueType,
};
use super::super::constructors;
use super::super::support::unsupported;
use super::SharedExpressionProvider;

pub(super) fn lower_enum_variant_condition_value(
    provider: &mut SharedExpressionProvider<'_>,
    subject: BackendValueId,
    variant: &str,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    constructors::lower_enum_variant_condition_value(provider, subject, variant, span, instructions)
}

pub(super) fn bind_enum_variant_payload(
    provider: &mut SharedExpressionProvider<'_>,
    subject: BackendValueId,
    variant: &str,
    payload: Option<&VariantPayload>,
    mode: CaseMode,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(), BackendContractError> {
    let Some(payload) = payload else {
        return Err(unsupported(span, "enum case payload binding"));
    };
    let binding = match payload {
        VariantPayload::Positional(items) if items.len() == 1 => Some(&items[0].name),
        VariantPayload::Named(items) if items.len() == 1 => Some(&items[0].binding.name),
        VariantPayload::Unit => None,
        _ => return Err(unsupported(span, "enum case payload shape")),
    };
    let Some(binding) = binding.filter(|name| *name != "_") else { return Ok(()) };
    let discriminant = enum_case_discriminant(provider, subject, variant, span)?;
    let payload_type = provider
        .enum_payload_types
        .get(&(subject, discriminant))
        .copied()
        .or_else(|| {
            provider
                .nominal_type(subject)
                .and_then(|type_name| provider.enum_variant_payload_type(type_name, variant))
        })
        .or_else(|| nominal_payload_type(provider, subject, discriminant))
        .ok_or_else(|| unsupported(span, "enum case payload source type"))?;
    let payload_nominal_type = nominal_payload_type_name(provider, subject, discriminant);
    let layout = neutral_enum_layout(provider, subject, span)?;
    let result = provider.allocate();
    provider.remember_value(result, payload_type);
    if let Some(payload_nominal_type) = payload_nominal_type {
        provider.remember_nominal_type(result, payload_nominal_type);
    }
    instructions.push(BackendInstruction::EnumPayloadBinding {
        result,
        subject,
        layout,
        payload_type,
        mode: match mode {
            CaseMode::Plain => BackendCaseMode::Plain,
            CaseMode::Abs => BackendCaseMode::Abs,
            CaseMode::Dat => BackendCaseMode::Dat,
        },
        span,
    });
    provider.bind_owner(binding, result, None, span, instructions)
}

fn enum_case_discriminant(
    provider: &SharedExpressionProvider<'_>,
    subject: BackendValueId,
    variant: &str,
    span: SourceSpan,
) -> Result<u32, BackendContractError> {
    let niche_option =
        matches!(provider.value_type(subject), Some(BackendValueType::NichePointerOption { .. }));
    let discriminant = match variant {
        "Ok" => Ok(0),
        "Err" => Ok(1),
        "Some" if niche_option => Ok(1),
        "None" if niche_option => Ok(0),
        "Some" => Ok(0),
        "None" => Ok(1),
        _ => {
            let layout = neutral_enum_layout(provider, subject, span)?;
            provider
                .nominal_type(subject)
                .and_then(|type_name| provider.enum_variant(type_name, variant))
                .or_else(|| provider.enum_variant_for_layout(layout, variant))
                .map(|(id, _)| id)
                .ok_or_else(|| unsupported(span, "unknown enum case variant"))
        }
    }?;
    Ok(discriminant)
}

fn nominal_payload_type(
    provider: &SharedExpressionProvider<'_>,
    subject: BackendValueId,
    discriminant: u32,
) -> Option<BackendValueType> {
    let type_name = provider.nominal_type(subject)?;
    let niche_option =
        matches!(provider.value_type(subject), Some(BackendValueType::NichePointerOption { .. }));
    let option_payload = type_name.name == "Option"
        && type_name.arguments.len() == 1
        && ((niche_option && discriminant == 1) || (!niche_option && discriminant == 0));
    let payload = if option_payload {
        type_name.arguments.first()
    } else if type_name.name == "Result" && type_name.arguments.len() == 2 && discriminant < 2 {
        type_name.arguments.get(discriminant as usize)
    } else {
        None
    }?;
    super::super::support::backend_value_type(provider.layouts, payload)
}

fn nominal_payload_type_name(
    provider: &SharedExpressionProvider<'_>,
    subject: BackendValueId,
    discriminant: u32,
) -> Option<crate::ast::TypeName> {
    let type_name = provider.nominal_type(subject)?;
    let niche_option =
        matches!(provider.value_type(subject), Some(BackendValueType::NichePointerOption { .. }));
    let argument = if type_name.name == "Option"
        && type_name.arguments.len() == 1
        && ((niche_option && discriminant == 1) || (!niche_option && discriminant == 0))
    {
        type_name.arguments.first()
    } else if type_name.name == "Result" && type_name.arguments.len() == 2 && discriminant < 2 {
        type_name.arguments.get(discriminant as usize)
    } else {
        None
    }?;
    Some(argument.clone())
}

fn neutral_enum_layout(
    provider: &SharedExpressionProvider<'_>,
    subject: BackendValueId,
    span: SourceSpan,
) -> Result<super::super::super::BackendEnumLayout, BackendContractError> {
    provider
        .value_type(subject)
        .and_then(|ty| match ty {
            BackendValueType::Enum { layout } => Some(layout),
            BackendValueType::NichePointerOption { layout, .. } => {
                Some(super::super::super::BackendEnumLayout {
                    layout,
                    discriminant_offset: 0,
                    discriminant_size: 4,
                    payload_offset: 0,
                    payload: None,
                    niche_pointer: true,
                })
            }
            _ => None,
        })
        .ok_or_else(|| {
            unsupported(
                span,
                format!(
                    "enum case subject is not a neutral enum value (subject type: {:?})",
                    provider.value_type(subject)
                ),
            )
        })
}
