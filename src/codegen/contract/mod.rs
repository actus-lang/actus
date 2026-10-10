mod cleanup;
mod errors;
mod instruction;

use crate::ast::Role;
use crate::lexer::SourceSpan;
use std::collections::BTreeSet;

use super::abi::BackendDirectAggregateAbi;
use super::abi::BackendEnumLayout;
pub use super::backend::BackendIntegerBinaryOp;
pub use super::backend::BackendIntegerCompareOp;
pub use super::backend::BackendIntegerUnaryOp;
use super::model::LoweredCleanupInstruction;
use super::symbols::{SymbolIdentity, SymbolKind};

use cleanup::validate_cleanup_operand;
pub use cleanup::{
    BackendCleanupArgument, BackendCleanupBridge, BackendCleanupBridgeDescriptor,
    BackendCleanupField, BackendCleanupOperand, BackendCleanupResult,
};
pub use instruction::BackendInstruction;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendEmitError(pub String);

impl std::fmt::Display for BackendEmitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for BackendEmitError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendAbiError(pub String);

impl std::fmt::Display for BackendAbiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for BackendAbiError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendValueType {
    Unit,
    Bool,
    Integer { signed: bool, width: u8 },
    Float { width: u8 },
    Pointer,
    Aggregate { layout: BackendLayout },
    Region { layout: BackendLayout },
    NichePointerOption { layout: BackendLayout },
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendPassingMode {
    Direct,
    DirectAggregate { abi: BackendDirectAggregateAbi },
    Indirect { layout: BackendLayout },
    Borrowed { layout: BackendLayout },
    ExclusiveLoan { layout: BackendLayout },
    OwnershipTransfer { layout: BackendLayout },
    RuntimeBridge { layout: BackendLayout },
}

impl BackendPassingMode {
    pub(super) fn layout(&self) -> Option<BackendLayout> {
        match self {
            Self::Direct => None,
            Self::DirectAggregate { abi } => Some(abi.layout),
            Self::Indirect { layout }
            | Self::Borrowed { layout }
            | Self::ExclusiveLoan { layout }
            | Self::OwnershipTransfer { layout }
            | Self::RuntimeBridge { layout } => Some(*layout),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendReturnMode {
    Direct,
    DirectAggregate { abi: BackendDirectAggregateAbi },
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
pub struct BackendFailure {
    pub result_type: SymbolIdentity,
    pub discriminant: u32,
    pub payload: Option<BackendValueId>,
    pub layout: BackendEnumLayout,
}

pub type BackendCleanupAction = LoweredCleanupInstruction;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendCleanupPlan {
    pub depth: usize,
    pub span: SourceSpan,
    pub actions: Vec<BackendCleanupAction>,
    pub bridges: Vec<Option<BackendCleanupBridgeDescriptor>>,
}

pub fn lower_semantic_cleanup(
    model: &crate::semantic::SemanticModel,
) -> Result<Vec<BackendCleanupPlan>, BackendContractError> {
    super::model::validate_cleanup_plans(model)
        .map_err(BackendContractError::InvalidCleanupContract)?;
    Ok(super::model::lower_cleanup_plans(model)
        .into_iter()
        .map(|plan| {
            let bridges = plan
                .instructions
                .iter()
                .map(|action| super::model::cleanup_bridge_descriptor(action, model))
                .collect();
            BackendCleanupPlan {
                depth: plan.depth,
                span: plan.span,
                actions: plan.instructions,
                bridges,
            }
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
    pub cleanup: Vec<BackendCleanupPlan>,
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
        validate_cleanup_plans(&self.cleanup)?;
        Ok(())
    }
}

fn validate_cleanup_plans(plans: &[BackendCleanupPlan]) -> Result<(), BackendContractError> {
    if plans.iter().any(|plan| plan.actions.len() != plan.bridges.len()) {
        return Err(BackendContractError::InvalidCleanupPlan);
    }
    Ok(())
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
    match &return_value.passing {
        BackendReturnMode::Direct => {}
        BackendReturnMode::DirectAggregate { abi } => {
            validate_direct_aggregate_type(return_value.ty, abi)?;
        }
        BackendReturnMode::ReturnSlot { layout } => {
            layout.validate()?;
            match return_value.ty {
                BackendValueType::Aggregate { layout: value_layout }
                | BackendValueType::Region { layout: value_layout }
                    if value_layout == *layout => {}
                _ => return Err(BackendContractError::ReturnSlotTypeMismatch),
            }
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
        BackendInstruction::Constant { ty, .. }
            if !matches!(ty, BackendValueType::Bool | BackendValueType::Integer { .. }) =>
        {
            Err(BackendContractError::InvalidValueType)
        }
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
        BackendInstruction::Fail { failure, .. } => super::abi::validate_failure(failure),
        BackendInstruction::Cleanup { action, operand, .. } => {
            validate_cleanup_operand(action, *operand)
        }
        BackendInstruction::MemoryCopy { size, alignment, .. }
            if *size == 0
                || *alignment == 0
                || !alignment.is_power_of_two()
                || size % alignment != 0 =>
        {
            Err(BackendContractError::InvalidMemoryCopy)
        }
        _ => Ok(()),
    }
}

fn validate_parameter(parameter: &BackendParameter) -> Result<(), BackendContractError> {
    if let Some(layout) = parameter.passing.layout() {
        layout.validate()?;
    }
    if let BackendPassingMode::DirectAggregate { abi } = &parameter.passing {
        validate_direct_aggregate_type(parameter.ty, abi)?;
    }
    let valid = match parameter.role {
        Role::Erg => matches!(
            &parameter.passing,
            BackendPassingMode::Direct
                | BackendPassingMode::DirectAggregate { .. }
                | BackendPassingMode::Indirect { .. }
                | BackendPassingMode::RuntimeBridge { .. }
        ),
        Role::Abs => matches!(
            &parameter.passing,
            BackendPassingMode::Direct
                | BackendPassingMode::DirectAggregate { .. }
                | BackendPassingMode::Indirect { .. }
                | BackendPassingMode::Borrowed { .. }
                | BackendPassingMode::RuntimeBridge { .. }
        ),
        Role::Ins => matches!(&parameter.passing, BackendPassingMode::ExclusiveLoan { .. }),
        Role::Dat => matches!(&parameter.passing, BackendPassingMode::OwnershipTransfer { .. }),
    };
    valid.then_some(()).ok_or(BackendContractError::RolePassingMismatch)
}

fn validate_direct_aggregate_type(
    ty: BackendValueType,
    abi: &BackendDirectAggregateAbi,
) -> Result<(), BackendContractError> {
    abi.validate()?;
    match ty {
        BackendValueType::Aggregate { layout } | BackendValueType::Region { layout }
            if layout == abi.layout =>
        {
            Ok(())
        }
        _ => Err(BackendContractError::DirectAggregateTypeMismatch),
    }
}

fn validate_value_type(ty: BackendValueType) -> Result<(), BackendContractError> {
    match ty {
        BackendValueType::Integer { width: 0, .. } | BackendValueType::Float { width: 0 } => {
            Err(BackendContractError::InvalidValueType)
        }
        BackendValueType::Aggregate { layout }
        | BackendValueType::Region { layout }
        | BackendValueType::NichePointerOption { layout } => layout.validate(),
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
    InvalidEnumLayout(&'static str),
    InvalidFunctionSymbol,
    InvalidExternalSymbol,
    MissingEntryBlock,
    EmptyParameterName,
    DuplicateParameterName,
    DuplicateBlockId,
    InvalidBlockReference,
    InvalidValueType,
    DirectAggregateTypeMismatch,
    InvalidFailureType,
    InvalidMemoryCopy,
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
