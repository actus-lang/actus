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
    Region { layout: BackendLayout },
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendReturn {
    pub role: Option<Role>,
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
    Fail {
        failure: BackendFailure,
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
pub struct BackendFailure {
    pub result_type: SymbolIdentity,
    pub discriminant: u32,
    pub payload: Option<BackendValueId>,
}

pub type BackendCleanupAction = NativeInstruction;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendCleanupPlan {
    pub depth: usize,
    pub span: SourceSpan,
    pub actions: Vec<BackendCleanupAction>,
}

pub fn lower_semantic_cleanup(
    model: &crate::semantic::SemanticModel,
) -> Result<Vec<BackendCleanupPlan>, BackendContractError> {
    super::model::validate_cleanup_plans(model)
        .map_err(BackendContractError::InvalidCleanupContract)?;
    Ok(super::model::lower_cleanup_plans(model)
        .into_iter()
        .map(|plan| BackendCleanupPlan {
            depth: plan.depth,
            span: plan.span,
            actions: plan.instructions,
        })
        .collect())
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendExternal {
    pub symbol: SymbolIdentity,
    pub parameters: Vec<BackendParameter>,
    pub return_value: BackendReturn,
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct BackendModule {
    pub externals: Vec<BackendExternal>,
    pub functions: Vec<BackendFunction>,
}

impl BackendModule {
    pub fn validate(&self) -> Result<(), BackendContractError> {
        let mut symbols = BTreeSet::new();
        for external in &self.externals {
            if external.symbol.kind() != SymbolKind::External {
                return Err(BackendContractError::InvalidExternalSymbol);
            }
            validate_signature(&external.parameters, &external.return_value)?;
            if !symbols.insert(external.symbol.clone()) {
                return Err(BackendContractError::DuplicateModuleSymbol);
            }
        }
        for function in &self.functions {
            function.validate()?;
            if !symbols.insert(function.symbol.clone()) {
                return Err(BackendContractError::DuplicateModuleSymbol);
            }
        }
        Ok(())
    }
}

impl BackendFunction {
    pub fn validate(&self) -> Result<(), BackendContractError> {
        validate_function_symbol(&self.symbol)?;
        validate_signature(&self.parameters, &self.return_value)?;
        let block_ids = validate_blocks(&self.blocks, self.entry)?;
        validate_instructions(&self.blocks, &block_ids)?;
        Ok(())
    }
}

fn validate_signature(
    parameters: &[BackendParameter],
    return_value: &BackendReturn,
) -> Result<(), BackendContractError> {
    validate_parameters(parameters)?;
    validate_return(return_value)
}

fn validate_function_symbol(symbol: &SymbolIdentity) -> Result<(), BackendContractError> {
    if matches!(symbol.kind(), SymbolKind::Verb | SymbolKind::External) {
        Ok(())
    } else {
        Err(BackendContractError::InvalidFunctionSymbol)
    }
}

fn validate_parameters(parameters: &[BackendParameter]) -> Result<(), BackendContractError> {
    let mut names = BTreeSet::new();
    for parameter in parameters {
        if parameter.name.is_empty() {
            return Err(BackendContractError::EmptyParameterName);
        }
        if !names.insert(&parameter.name) {
            return Err(BackendContractError::DuplicateParameterName);
        }
        validate_value_type(parameter.ty)?;
        validate_parameter(parameter)?;
    }
    Ok(())
}

fn validate_return(return_value: &BackendReturn) -> Result<(), BackendContractError> {
    validate_value_type(return_value.ty)?;
    if let BackendReturnMode::ReturnSlot { layout } = return_value.passing {
        layout.validate()?;
        match return_value.ty {
            BackendValueType::Aggregate { layout: value_layout }
            | BackendValueType::Region { layout: value_layout }
                if value_layout == layout => {}
            _ => return Err(BackendContractError::ReturnSlotTypeMismatch),
        }
    }
    Ok(())
}

fn validate_blocks(
    blocks: &[BackendBlock],
    entry: BackendBlockId,
) -> Result<BTreeSet<BackendBlockId>, BackendContractError> {
    let mut ids = BTreeSet::new();
    for block in blocks {
        if !ids.insert(block.id) {
            return Err(BackendContractError::DuplicateBlockId);
        }
    }
    if !ids.contains(&entry) {
        return Err(BackendContractError::MissingEntryBlock);
    }
    Ok(ids)
}

fn validate_instructions(
    blocks: &[BackendBlock],
    block_ids: &BTreeSet<BackendBlockId>,
) -> Result<(), BackendContractError> {
    for block in blocks {
        for instruction in &block.instructions {
            validate_instruction(instruction, block_ids)?;
        }
    }
    Ok(())
}

fn validate_instruction(
    instruction: &BackendInstruction,
    block_ids: &BTreeSet<BackendBlockId>,
) -> Result<(), BackendContractError> {
    match instruction {
        BackendInstruction::Call { callee, .. } if callee.as_str().is_empty() => {
            Err(BackendContractError::InvalidCallSymbol)
        }
        BackendInstruction::Branch { then_block, else_block, .. }
            if !block_ids.contains(then_block) || !block_ids.contains(else_block) =>
        {
            Err(BackendContractError::InvalidBlockReference)
        }
        BackendInstruction::Jump { target, .. } if !block_ids.contains(target) => {
            Err(BackendContractError::InvalidBlockReference)
        }
        BackendInstruction::Fail { failure, .. }
            if failure.result_type.kind() != SymbolKind::Enum =>
        {
            Err(BackendContractError::InvalidFailureType)
        }
        _ => Ok(()),
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
        BackendValueType::Aggregate { layout } | BackendValueType::Region { layout } => {
            layout.validate()
        }
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
    InvalidExternalSymbol,
    MissingEntryBlock,
    EmptyParameterName,
    DuplicateParameterName,
    DuplicateBlockId,
    InvalidBlockReference,
    InvalidValueType,
    InvalidFailureType,
    InvalidCleanupContract(String),
    ReturnSlotTypeMismatch,
    InvalidCallSymbol,
    RolePassingMismatch,
    DuplicateModuleSymbol,
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
            Self::InvalidExternalSymbol => formatter.write_str("invalid backend external symbol"),
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
            Self::InvalidFailureType => {
                formatter.write_str("backend typed failure must reference an enum result type")
            }
            Self::InvalidCleanupContract(reason) => {
                write!(formatter, "invalid semantic cleanup contract: {reason}")
            }
            Self::ReturnSlotTypeMismatch => {
                formatter.write_str("backend return slot must match an aggregate return type")
            }
            Self::InvalidCallSymbol => formatter.write_str("backend call has an empty symbol"),
            Self::RolePassingMismatch => {
                formatter.write_str("backend parameter role and passing mode disagree")
            }
            Self::DuplicateModuleSymbol => {
                formatter.write_str("backend module contains a duplicate symbol")
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
