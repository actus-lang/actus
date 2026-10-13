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
    let discriminant = match variant {
        "Ok" => 0,
        "Err" => 1,
        "Some" => 1,
        "None" => 0,
        _ => return Err(unsupported(span, "unknown enum case variant")),
    };
    let payload_type = provider
        .enum_payload_types
        .get(&(subject, discriminant))
        .copied()
        .or_else(|| {
            let type_name = provider.nominal_type(subject)?;
            (type_name.name == "Option" && type_name.arguments.len() == 1 && discriminant == 1)
                .then(|| {
                    super::super::support::backend_value_type(
                        provider.layouts,
                        &type_name.arguments[0],
                    )
                })
                .flatten()
        })
        .ok_or_else(|| unsupported(span, "enum case payload source type"))?;
    let layout = neutral_enum_layout(provider, subject, span)?;
    let result = provider.allocate();
    provider.remember_value(result, payload_type);
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

fn neutral_enum_layout(
    provider: &SharedExpressionProvider<'_>,
    subject: BackendValueId,
    span: SourceSpan,
) -> Result<super::super::super::BackendEnumLayout, BackendContractError> {
    provider
        .value_type(subject)
        .and_then(|ty| match ty {
            BackendValueType::Enum { layout } => Some(layout),
            BackendValueType::NichePointerOption { layout } => {
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
