use crate::ast::VerbDecl;

#[derive(Debug)]
pub struct NativeAbiError(String);

impl std::fmt::Display for NativeAbiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for NativeAbiError {}

pub fn validate_integer_return(verb: &VerbDecl) -> Result<(), NativeAbiError> {
    let Some(return_type) = &verb.return_type else {
        return Ok(());
    };
    if return_type.name == "Int" {
        Ok(())
    } else {
        Err(NativeAbiError(format!(
            "native integer slice supports `Int` returns, found `{}`",
            return_type.name
        )))
    }
}
