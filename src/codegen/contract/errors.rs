use crate::lexer::SourceSpan;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendContractError {
    InvalidLayout(&'static str),
    InvalidEnumLayout(&'static str),
    InvalidFunctionSymbol,
    InvalidExternalSymbol,
    MissingEntryBlock,
    EmptyParameterName,
    DuplicateParameterName,
    DuplicateBlockId,
    InvalidBlockReference,
    InvalidBlockArguments(&'static str),
    InvalidValueType,
    DirectAggregateTypeMismatch,
    InvalidFailureType,
    InvalidMemoryCopy,
    InvalidAggregateInstruction,
    InvalidAggregateFieldAccess,
    InvalidAggregateAbi(&'static str),
    InvalidCleanupContract(String),
    InvalidCleanupPlan,
    InvalidCleanupOperand,
    ReturnSlotTypeMismatch,
    InvalidCallSymbol,
    RolePassingMismatch,
    DuplicateModuleSymbol,
    Unsupported(BackendUnsupportedOperation),
}

impl BackendContractError {
    pub fn with_backend_context(
        mut self,
        backend: impl Into<String>,
        target: impl Into<String>,
    ) -> Self {
        if let Self::Unsupported(operation) = &mut self {
            operation.context =
                BackendTargetContext { backend: backend.into(), target: target.into() };
        }
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendTargetContext {
    pub backend: String,
    pub target: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendUnsupportedOperation {
    pub operation: String,
    pub span: SourceSpan,
    pub context: BackendTargetContext,
    pub reason: String,
}

impl BackendUnsupportedOperation {
    pub fn new(
        operation: impl Into<String>,
        span: SourceSpan,
        context: BackendTargetContext,
        reason: impl Into<String>,
    ) -> Self {
        Self { operation: operation.into(), span, context, reason: reason.into() }
    }
}

impl std::fmt::Display for BackendContractError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(message) = simple_message(self) {
            return formatter.write_str(message);
        }
        match self {
            Self::InvalidLayout(reason) => write!(formatter, "invalid backend layout: {reason}"),
            Self::InvalidEnumLayout(reason) => {
                write!(formatter, "invalid backend enum layout: {reason}")
            }
            Self::InvalidAggregateAbi(reason) => {
                write!(formatter, "invalid direct aggregate ABI: {reason}")
            }
            Self::InvalidCleanupContract(reason) => {
                write!(formatter, "invalid semantic cleanup contract: {reason}")
            }
            Self::Unsupported(operation) => write!(
                formatter,
                "backend `{}` does not support `{}` for target `{}` at {:?}: {}",
                operation.context.backend,
                operation.operation,
                operation.context.target,
                operation.span,
                operation.reason
            ),
            _ => unreachable!("simple backend contract error was not formatted"),
        }
    }
}

fn simple_message(error: &BackendContractError) -> Option<&'static str> {
    Some(match error {
        BackendContractError::InvalidFunctionSymbol => "invalid backend function symbol",
        BackendContractError::InvalidExternalSymbol => "invalid backend external symbol",
        BackendContractError::MissingEntryBlock => "backend function is missing its entry block",
        BackendContractError::EmptyParameterName => "backend parameter name is empty",
        BackendContractError::DuplicateParameterName => {
            "backend function has duplicate parameter names"
        }
        BackendContractError::DuplicateBlockId => "backend function has duplicate block ids",
        BackendContractError::InvalidBlockReference => {
            "backend control flow references an unknown block"
        }
        BackendContractError::InvalidBlockArguments(reason) => reason,
        BackendContractError::InvalidValueType => "backend value type has an invalid width",
        BackendContractError::DirectAggregateTypeMismatch => {
            "direct aggregate ABI descriptor does not match the value type"
        }
        BackendContractError::InvalidFailureType => {
            "backend typed failure must reference an enum result type"
        }
        BackendContractError::InvalidMemoryCopy => {
            "backend memory copy requires a non-zero aligned extent"
        }
        BackendContractError::InvalidAggregateInstruction => {
            "backend aggregate construction has invalid field layout"
        }
        BackendContractError::InvalidAggregateFieldAccess => {
            "backend aggregate field access has invalid layout or value type"
        }
        BackendContractError::ReturnSlotTypeMismatch => {
            "backend return slot must match an aggregate return type"
        }
        BackendContractError::InvalidCleanupOperand => {
            "runtime cleanup requires an explicit lowered value operand"
        }
        BackendContractError::InvalidCleanupPlan => {
            "backend cleanup plan actions and bridge descriptors must have equal length"
        }
        BackendContractError::InvalidCallSymbol => "backend call has an empty symbol",
        BackendContractError::RolePassingMismatch => {
            "backend parameter role and passing mode disagree"
        }
        BackendContractError::DuplicateModuleSymbol => "backend module contains a duplicate symbol",
        _ => return None,
    })
}

impl std::error::Error for BackendContractError {}
