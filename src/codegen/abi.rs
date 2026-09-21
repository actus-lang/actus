use crate::ast::{ExternalVerbDecl, VerbDecl};

use super::types::NativeType;

#[derive(Debug)]
pub struct NativeAbiError(String);

impl std::fmt::Display for NativeAbiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for NativeAbiError {}

pub fn validate_native_signature(verb: &VerbDecl) -> Result<(), NativeAbiError> {
    if let Some(return_type) = &verb.return_type
        && NativeType::from_name(&return_type.name).is_none()
    {
        return Err(NativeAbiError(format!(
            "native backend supports `Int` and `Buffer` returns, found `{}`",
            return_type.name
        )));
    }
    if let Some(parameter) =
        verb.params.iter().find(|parameter| NativeType::from_name(&parameter.ty.name).is_none())
    {
        return Err(NativeAbiError(format!(
            "native backend supports `Int` and `Buffer` parameters, found `{}` for `{}`",
            parameter.ty.name, parameter.name
        )));
    }
    Ok(())
}

pub fn validate_external_native_signature(verb: &ExternalVerbDecl) -> Result<(), NativeAbiError> {
    if verb.return_type.is_none() {
        return Err(NativeAbiError(format!(
            "external native verb `{}` requires an explicit return type",
            verb.name
        )));
    }
    if let Some(parameter) =
        verb.params.iter().find(|parameter| NativeType::from_name(&parameter.ty.name).is_none())
    {
        return Err(NativeAbiError(format!(
            "native backend supports `Int` and `Buffer` parameters, found `{}` for `{}`",
            parameter.ty.name, parameter.name
        )));
    }
    if let Some(return_type) = &verb.return_type
        && NativeType::from_name(&return_type.name).is_none()
    {
        return Err(NativeAbiError(format!(
            "native backend supports `Int` and `Buffer` returns, found `{}`",
            return_type.name
        )));
    }
    Ok(())
}
