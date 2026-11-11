use crate::ast::{CaseMode, VariantPayload};
use crate::lexer::SourceSpan;

use super::super::super::{
    BackendCaseMode, BackendContractError, BackendExpressionProvider, BackendInstruction,
    BackendValueId, BackendValueType,
};
use super::super::constructors;
use super::super::errors::unsupported;
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
    enum_case_discriminant(provider, subject, variant, span)?;
    let layout = neutral_enum_layout(provider, subject, span)?;
    let fields = provider
        .nominal_type(subject)
        .and_then(|type_name| provider.enum_variant_payload_fields(type_name, variant))
        .ok_or_else(|| unsupported(span, "enum case payload fields"))?;
    let bindings = match payload {
        VariantPayload::Positional(items) => items
            .iter()
            .enumerate()
            .map(|(index, item)| (index.to_string(), item.name.clone()))
            .collect::<Vec<_>>(),
        VariantPayload::Named(items) => items
            .iter()
            .map(|item| (item.name.clone(), item.binding.name.clone()))
            .collect::<Vec<_>>(),
        VariantPayload::Unit => return Ok(()),
    };
    if bindings.len() != fields.len() {
        return Err(unsupported(span, "enum case payload shape"));
    }
    for (field_name, binding) in bindings {
        if binding == "_" {
            continue;
        }
        bind_enum_field(
            provider,
            subject,
            layout,
            &fields,
            &field_name,
            &binding,
            mode,
            span,
            instructions,
        )?;
    }
    Ok(())
}

fn bind_enum_field(
    provider: &mut SharedExpressionProvider<'_>,
    subject: BackendValueId,
    layout: super::super::super::BackendEnumLayout,
    fields: &[super::super::super::BackendEnumPayloadField],
    field_name: &str,
    binding: &str,
    mode: CaseMode,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(), BackendContractError> {
    let field = fields
        .iter()
        .find(|field| field.name == field_name)
        .ok_or_else(|| unsupported(span, "enum case payload field"))?;
    let payload_type = provider
        .backend_value_type(&field.ty)
        .ok_or_else(|| unsupported(span, "enum case payload source type"))?;
    let result = provider.allocate();
    provider.remember_value(result, payload_type);
    provider.remember_nominal_type(result, field.ty.clone());
    instructions.push(BackendInstruction::EnumPayloadBinding {
        result,
        subject,
        layout,
        payload_type,
        payload_offset: field.offset,
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

fn neutral_enum_layout(
    provider: &SharedExpressionProvider<'_>,
    subject: BackendValueId,
    span: SourceSpan,
) -> Result<super::super::super::BackendEnumLayout, BackendContractError> {
    let layout = provider
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
        .or_else(|| {
            provider.nominal_type(subject).and_then(|type_name| provider.enum_layout(type_name))
        });
    layout.ok_or_else(|| {
        unsupported(
            span,
            format!(
                "enum case subject is not a neutral enum value (subject type: {:?})",
                provider.value_type(subject)
            ),
        )
    })
}
