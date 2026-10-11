use std::collections::BTreeMap;

use crate::ast::VerbDecl;

use super::super::super::{
    BackendContractError, BackendInstruction, BackendValueId, BackendValueType,
};
use super::super::support::{backend_value_type, scalar_backend_type_name, unsupported};
use super::SharedExpressionProvider;

pub(super) fn set_binding_value(
    provider: &mut SharedExpressionProvider<'_>,
    name: &str,
    value: super::super::super::BackendValueId,
) {
    if let Some(scope_index) = provider
        .locals
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, scope)| scope.contains_key(name).then_some(index))
    {
        provider.locals[scope_index].insert(name.to_owned(), value);
    } else if provider.parameters.contains_key(name) {
        provider.parameters.insert(name.to_owned(), value);
    }
}

pub(super) fn parameter_types(
    verb: &VerbDecl,
    layouts: &impl super::super::super::BackendLayoutProvider,
) -> BTreeMap<BackendValueId, BackendValueType> {
    verb.params
        .iter()
        .enumerate()
        .filter_map(|(index, parameter)| {
            super::super::super::signature::lower_parameter(parameter, layouts)
                .ok()
                .map(|parameter| (BackendValueId(index as u32), parameter.ty))
        })
        .collect()
}

pub(super) fn enum_payload_types(
    verb: &VerbDecl,
    layouts: &impl super::super::super::BackendLayoutProvider,
) -> BTreeMap<(BackendValueId, u32), BackendValueType> {
    verb.params
        .iter()
        .enumerate()
        .filter(|(_, parameter)| parameter.ty.name == "Result" && parameter.ty.arguments.len() == 2)
        .flat_map(|(index, parameter)| {
            parameter.ty.arguments.iter().enumerate().filter_map(move |(discriminant, payload)| {
                backend_value_type(layouts, payload).map(|payload_type| {
                    ((BackendValueId(index as u32), discriminant as u32), payload_type)
                })
            })
        })
        .collect()
}

pub(super) fn coerce_owner_value(
    provider: &mut SharedExpressionProvider<'_>,
    value: BackendValueId,
    declared_type: Option<&str>,
    span: crate::lexer::SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let Some(declared_type) = declared_type else { return Ok(value) };
    let Some(target_type) = scalar_backend_type_name(declared_type) else { return Ok(value) };
    let source_type = provider
        .value_type(value)
        .ok_or_else(|| unsupported(span, "owner initializer type is unavailable"))?;
    if source_type == target_type {
        return Ok(value);
    }
    let converted = provider.allocate();
    provider.remember_value(converted, target_type);
    instructions.push(BackendInstruction::IntegerCast {
        result: converted,
        source: value,
        source_type,
        target_type,
        span,
    });
    Ok(converted)
}

pub(super) fn coerce_value_to_type(
    provider: &mut SharedExpressionProvider<'_>,
    value: BackendValueId,
    target_type: Option<BackendValueType>,
    span: crate::lexer::SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let Some(target_type) = target_type else { return Ok(value) };
    let source_type = provider
        .value_type(value)
        .ok_or_else(|| unsupported(span, "return value type is unavailable"))?;
    if source_type == target_type {
        return Ok(value);
    }
    if !matches!(source_type, BackendValueType::Integer { .. })
        || !matches!(target_type, BackendValueType::Integer { .. })
    {
        return Ok(value);
    }
    let converted = provider.allocate();
    provider.remember_value(converted, target_type);
    instructions.push(BackendInstruction::IntegerCast {
        result: converted,
        source: value,
        source_type,
        target_type,
        span,
    });
    Ok(converted)
}
