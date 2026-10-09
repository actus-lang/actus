use crate::ast::Role;
use crate::lexer::SourceSpan;
use std::collections::BTreeSet;

use super::model::NativeInstruction;
use super::symbols::{SymbolIdentity, SymbolKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendValueType {
    Unit,
    Bool,
    Integer { signed: bool, width: u8 },
    Float { width: u8 },
    Pointer,
    Aggregate { layout: BackendLayout },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackendLayout {
    pub size: u32,
    pub alignment: u32,
}

impl BackendLayout {
    pub fn validate(self) -> Result<(), BackendContractError> {
        if self.alignment == 0 || !self.alignment.is_power_of_two() {
            return Err(BackendContractError::InvalidLayout("alignment must be a power of two"));
        }
        if self.size % self.alignment != 0 {
            return Err(BackendContractError::InvalidLayout(
                "aggregate size must be aligned to its declared alignment",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendParameter {
    pub name: String,
    pub role: Role,
    pub ty: BackendValueType,
    pub passing: BackendPassingMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendPassingMode {
    Direct,
    Indirect { layout: BackendLayout },
    Borrowed { layout: BackendLayout },
    ExclusiveLoan { layout: BackendLayout },
    OwnershipTransfer { layout: BackendLayout },
    RuntimeBridge { layout: BackendLayout },
}

impl BackendPassingMode {
    fn layout(self) -> Option<BackendLayout> {
        match self {
            Self::Direct => None,
            Self::Indirect { layout }
            | Self::Borrowed { layout }
            | Self::ExclusiveLoan { layout }
            | Self::OwnershipTransfer { layout }
            | Self::RuntimeBridge { layout } => Some(layout),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendReturnMode {
    Direct,
    ReturnSlot { layout: BackendLayout },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackendReturn {
    pub ty: BackendValueType,
    pub passing: BackendReturnMode,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct BackendValueId(pub u32);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct BackendBlockId(pub u32);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendInstruction {
    Call {
        callee: SymbolIdentity,
        arguments: Vec<BackendValueId>,
        result: Option<BackendValueId>,
        span: SourceSpan,
    },
    Branch {
        condition: BackendValueId,
        then_block: BackendBlockId,
        else_block: BackendBlockId,
        span: SourceSpan,
    },
    Jump {
        target: BackendBlockId,
        span: SourceSpan,
    },
    Return {
        value: Option<BackendValueId>,
        span: SourceSpan,
    },
    Trap {
        reason: String,
        span: SourceSpan,
    },
    Cleanup {
        action: NativeInstruction,
        span: SourceSpan,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendBlock {
    pub id: BackendBlockId,
    pub instructions: Vec<BackendInstruction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendFunction {
    pub symbol: SymbolIdentity,
    pub parameters: Vec<BackendParameter>,
    pub return_value: BackendReturn,
    pub entry: BackendBlockId,
    pub blocks: Vec<BackendBlock>,
}

impl BackendFunction {
    pub fn validate(&self) -> Result<(), BackendContractError> {
        if self.symbol.kind() != SymbolKind::Verb && self.symbol.kind() != SymbolKind::External {
            return Err(BackendContractError::InvalidFunctionSymbol);
        }
        let mut block_ids = BTreeSet::new();
        let mut parameter_names = BTreeSet::new();
        for parameter in &self.parameters {
            if parameter.name.is_empty() {
                return Err(BackendContractError::EmptyParameterName);
            }
            if !parameter_names.insert(&parameter.name) {
                return Err(BackendContractError::DuplicateParameterName);
            }
            validate_value_type(parameter.ty)?;
            validate_parameter(parameter)?;
        }
        validate_value_type(self.return_value.ty)?;
        if let BackendReturnMode::ReturnSlot { layout } = self.return_value.passing {
            layout.validate()?;
            match self.return_value.ty {
                BackendValueType::Aggregate { layout: value_layout } if value_layout == layout => {}
                _ => return Err(BackendContractError::ReturnSlotTypeMismatch),
            }
        }
        for block in &self.blocks {
            if !block_ids.insert(block.id) {
                return Err(BackendContractError::DuplicateBlockId);
            }
        }
        if self.blocks.iter().filter(|block| block.id == self.entry).count() != 1 {
            return Err(BackendContractError::MissingEntryBlock);
        }
        for block in &self.blocks {
            for instruction in &block.instructions {
                match instruction {
                    BackendInstruction::Call { callee, .. } => {
                        if callee.as_str().is_empty() {
                            return Err(BackendContractError::InvalidCallSymbol);
                        }
                    }
                    BackendInstruction::Branch { then_block, else_block, .. } => {
                        if !block_ids.contains(then_block) || !block_ids.contains(else_block) {
                            return Err(BackendContractError::InvalidBlockReference);
                        }
                    }
                    BackendInstruction::Jump { target, .. } => {
                        if !block_ids.contains(target) {
                            return Err(BackendContractError::InvalidBlockReference);
                        }
                    }
                    BackendInstruction::Return { .. }
                    | BackendInstruction::Trap { .. }
                    | BackendInstruction::Cleanup { .. } => {}
                }
            }
        }
        Ok(())
    }
}

fn validate_parameter(parameter: &BackendParameter) -> Result<(), BackendContractError> {
    if let Some(layout) = parameter.passing.layout() {
        layout.validate()?;
    }
    let valid = match parameter.role {
        Role::Erg => matches!(
            parameter.passing,
            BackendPassingMode::Direct
                | BackendPassingMode::Indirect { .. }
                | BackendPassingMode::RuntimeBridge { .. }
        ),
        Role::Abs => matches!(
            parameter.passing,
            BackendPassingMode::Direct
                | BackendPassingMode::Indirect { .. }
                | BackendPassingMode::Borrowed { .. }
                | BackendPassingMode::RuntimeBridge { .. }
        ),
        Role::Ins => matches!(parameter.passing, BackendPassingMode::ExclusiveLoan { .. }),
        Role::Dat => matches!(parameter.passing, BackendPassingMode::OwnershipTransfer { .. }),
    };
    valid.then_some(()).ok_or(BackendContractError::RolePassingMismatch)
}

fn validate_value_type(ty: BackendValueType) -> Result<(), BackendContractError> {
    match ty {
        BackendValueType::Integer { width: 0, .. } | BackendValueType::Float { width: 0 } => {
            Err(BackendContractError::InvalidValueType)
        }
        BackendValueType::Aggregate { layout } => layout.validate(),
        BackendValueType::Unit
        | BackendValueType::Bool
        | BackendValueType::Integer { .. }
        | BackendValueType::Float { .. }
        | BackendValueType::Pointer => Ok(()),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendContractError {
    InvalidLayout(&'static str),
    InvalidFunctionSymbol,
    MissingEntryBlock,
    EmptyParameterName,
    DuplicateParameterName,
    DuplicateBlockId,
    InvalidBlockReference,
    InvalidValueType,
    ReturnSlotTypeMismatch,
    InvalidCallSymbol,
    RolePassingMismatch,
    Unsupported(BackendUnsupportedOperation),
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
        match self {
            Self::InvalidLayout(reason) => write!(formatter, "invalid backend layout: {reason}"),
            Self::InvalidFunctionSymbol => formatter.write_str("invalid backend function symbol"),
            Self::MissingEntryBlock => {
                formatter.write_str("backend function is missing its entry block")
            }
            Self::EmptyParameterName => formatter.write_str("backend parameter name is empty"),
            Self::DuplicateParameterName => {
                formatter.write_str("backend function has duplicate parameter names")
            }
            Self::DuplicateBlockId => {
                formatter.write_str("backend function has duplicate block ids")
            }
            Self::InvalidBlockReference => {
                formatter.write_str("backend control flow references an unknown block")
            }
            Self::InvalidValueType => {
                formatter.write_str("backend value type has an invalid width")
            }
            Self::ReturnSlotTypeMismatch => {
                formatter.write_str("backend return slot must match an aggregate return type")
            }
            Self::InvalidCallSymbol => formatter.write_str("backend call has an empty symbol"),
            Self::RolePassingMismatch => {
                formatter.write_str("backend parameter role and passing mode disagree")
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
        }
    }
}

impl std::error::Error for BackendContractError {}
