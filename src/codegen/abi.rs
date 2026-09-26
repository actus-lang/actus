use crate::ast::{DispatchMode, ExternalVerbDecl, VerbDecl};

use super::layout::LayoutRegistry;
use super::types::NativeType;

#[derive(Debug)]
pub struct NativeAbiError(String);

impl std::fmt::Display for NativeAbiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for NativeAbiError {}

pub fn validate_native_signature(
    verb: &VerbDecl,
    layouts: &LayoutRegistry,
) -> Result<(), NativeAbiError> {
    if let Some(return_type) = &verb.return_type
        && NativeType::try_from_type_name_with_layout(Some(&return_type.ty), layouts).is_none()
    {
        return Err(NativeAbiError(format!(
            "native backend cannot lower return type `{}`",
            return_type.ty.name
        )));
    }
    if let Some(parameter) = verb.params.iter().find(|parameter| {
        parameter.dispatch != DispatchMode::Dynamic
            && NativeType::try_from_type_name_with_layout(Some(&parameter.ty), layouts).is_none()
    }) {
        return Err(NativeAbiError(format!(
            "native backend cannot lower parameter `{}` of type `{}`",
            parameter.name, parameter.ty.name
        )));
    }
    Ok(())
}

pub fn validate_external_native_signature(
    verb: &ExternalVerbDecl,
    layouts: &LayoutRegistry,
) -> Result<(), NativeAbiError> {
    if !verb.unsafe_boundary {
        return Err(NativeAbiError(format!(
            "external native verb `{}` requires an explicit `unsafe` boundary",
            verb.name
        )));
    }
    if verb.return_type.is_none() {
        return Err(NativeAbiError(format!(
            "external native verb `{}` requires an explicit return type",
            verb.name
        )));
    }
    if let Some(parameter) = verb.params.iter().find(|parameter| {
        NativeType::try_from_type_name_with_layout(Some(&parameter.ty), layouts).is_none()
    }) {
        return Err(NativeAbiError(format!(
            "native backend cannot lower parameter `{}` of type `{}`",
            parameter.name, parameter.ty.name
        )));
    }
    if let Some(return_type) = &verb.return_type
        && NativeType::try_from_type_name_with_layout(Some(&return_type.ty), layouts).is_none()
    {
        return Err(NativeAbiError(format!(
            "native backend cannot lower return type `{}`",
            return_type.ty.name
        )));
    }
    Ok(())
}
