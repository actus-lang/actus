use crate::ast::VerbDecl;

#[derive(Debug)]
pub struct NativeAbiError(String);

impl std::fmt::Display for NativeAbiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for NativeAbiError {}

pub fn validate_integer_signature(verb: &VerbDecl) -> Result<(), NativeAbiError> {
    if let Some(return_type) = &verb.return_type
        && return_type.name != "Int"
    {
        Err(NativeAbiError(format!(
            "native integer slice supports `Int` returns, found `{}`",
            return_type.name
        )))
    } else if let Some(parameter) = verb.params.iter().find(|parameter| parameter.ty.name != "Int")
    {
        Err(NativeAbiError(format!(
            "native integer slice supports `Int` parameters, found `{}` for `{}`",
            parameter.ty.name, parameter.name
        )))
    } else {
        Ok(())
    }
}
