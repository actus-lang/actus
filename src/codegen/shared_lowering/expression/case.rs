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
        _ => return Err(unsupported(span, "unknown Result case variant")),
    };
    let Some(payload_type) = provider.enum_payload_types.get(&(subject, discriminant)).copied()
    else {
        return Err(unsupported(span, "enum case payload source type"));
    };
    let layout = provider
        .value_types
        .get(&subject)
        .and_then(|ty| match ty {
            BackendValueType::Enum { layout } => Some(*layout),
            _ => None,
        })
        .ok_or_else(|| unsupported(span, "enum case subject is not a neutral enum value"))?;
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
