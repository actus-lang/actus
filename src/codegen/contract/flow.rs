use crate::lexer::SourceSpan;

use super::{BackendContractError, BackendValueId, BackendValueType};

/// A value accepted by a backend-neutral basic block.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackendBlockParameter {
    pub value: BackendValueId,
    pub ty: BackendValueType,
    pub span: SourceSpan,
}

/// A value supplied by one predecessor edge to a block parameter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackendEdgeArgument {
    pub value: BackendValueId,
    pub ty: BackendValueType,
    pub span: SourceSpan,
}

/// Checks one predecessor edge before backend-specific control-flow lowering.
pub fn validate_edge_arguments(
    parameters: &[BackendBlockParameter],
    arguments: &[BackendEdgeArgument],
) -> Result<(), BackendContractError> {
    if parameters.len() != arguments.len() {
        return Err(BackendContractError::InvalidBlockArguments(
            "edge argument count does not match block parameters",
        ));
    }
    if parameters.iter().zip(arguments).any(|(parameter, argument)| parameter.ty != argument.ty) {
        return Err(BackendContractError::InvalidBlockArguments(
            "edge argument type does not match its block parameter",
        ));
    }
    Ok(())
}
