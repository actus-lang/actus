mod cleanup;
mod errors;
mod flow;
mod instruction;
mod semantic_cleanup;
mod validation;

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

pub use cleanup::{
    BackendCleanupArgument, BackendCleanupBridge, BackendCleanupBridgeDescriptor,
    BackendCleanupField, BackendCleanupOperand, BackendCleanupProvider,
    BackendCleanupResolvedDescriptor, BackendCleanupResult,
};
pub use errors::{BackendContractError, BackendTargetContext, BackendUnsupportedOperation};
pub use flow::{BackendBlockParameter, BackendEdgeArgument, validate_edge_arguments};
pub use instruction::{BackendAggregateField, BackendInstruction};
pub use semantic_cleanup::lower_semantic_cleanup_with_provider;

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
    Enum { layout: BackendEnumLayout },
    NichePointerOption { layout: BackendLayout, payload_layout: Option<BackendLayout> },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendCaseMode {
    Plain,
    Abs,
    Dat,
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
        if !self.size.is_multiple_of(self.alignment) {
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
    ExclusiveScalarLoan { ty: BackendValueType },
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
            Self::ExclusiveScalarLoan { .. } => None,
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
pub enum BackendCleanupOrigin {
    Scope,
    Explicit { span: SourceSpan },
    Return { span: SourceSpan },
    Loop { span: SourceSpan, kind: crate::semantic::LoopExitKind },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendCleanupPlan {
    pub depth: usize,
    pub span: SourceSpan,
    pub origin: BackendCleanupOrigin,
    pub actions: Vec<BackendCleanupAction>,
    pub bridges: Vec<Option<BackendCleanupBridgeDescriptor>>,
    pub resolved_bridges: Vec<Vec<BackendCleanupResolvedDescriptor>>,
}

pub fn lower_semantic_cleanup(
    model: &crate::semantic::SemanticModel,
) -> Result<Vec<BackendCleanupPlan>, BackendContractError> {
    super::model::validate_cleanup_plans(model)
        .map_err(BackendContractError::InvalidCleanupContract)?;
    Ok(super::model::lower_cleanup_plans(model)
        .into_iter()
        .map(|plan| {
            let action_count = plan.instructions.len();
            let bridges = plan
                .instructions
                .iter()
                .map(|action| super::model::cleanup_bridge_descriptor(action, model))
                .collect();
            BackendCleanupPlan {
                depth: plan.depth,
                span: plan.span,
                origin: BackendCleanupOrigin::Scope,
                actions: plan.instructions,
                bridges,
                resolved_bridges: vec![Vec::new(); action_count],
            }
        })
        .collect())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendBlock {
    pub id: BackendBlockId,
    pub parameters: Vec<BackendBlockParameter>,
    pub instructions: Vec<BackendInstruction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendFunction {
    pub symbol: SymbolIdentity,
    pub parameters: Vec<BackendParameter>,
    pub return_value: BackendReturn,
    pub hosted_entry_return: bool,
    pub entry: BackendBlockId,
    pub blocks: Vec<BackendBlock>,
    pub cleanup: Vec<BackendCleanupPlan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendExternal {
    pub symbol: SymbolIdentity,
    pub parameters: Vec<BackendParameter>,
    pub return_value: BackendReturn,
    pub runtime_pointer_return: bool,
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
        validation::validate_instructions(&self.blocks, &block_ids)?;
        validate_cleanup_plans(&self.cleanup)?;
        Ok(())
    }
}

fn validate_cleanup_plans(plans: &[BackendCleanupPlan]) -> Result<(), BackendContractError> {
    if plans.iter().any(|plan| {
        plan.actions.len() != plan.bridges.len()
            || plan.actions.len() != plan.resolved_bridges.len()
    }) {
        return Err(BackendContractError::InvalidCleanupPlan);
    }
    for plan in plans {
        for descriptor in plan.resolved_bridges.iter().flatten() {
            descriptor.validate()?;
        }
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
    if matches!(symbol.kind(), SymbolKind::Verb | SymbolKind::External | SymbolKind::Performance) {
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
                | BackendValueType::Enum {
                    layout: BackendEnumLayout { layout: value_layout, .. },
                } if value_layout == *layout => {}
                BackendValueType::NichePointerOption { payload_layout, .. }
                    if payload_layout.is_some_and(|payload_layout| payload_layout == *layout) => {}
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
        let mut parameter_values = BTreeSet::new();
        for parameter in &block.parameters {
            validate_value_type(parameter.ty)?;
            if !parameter_values.insert(parameter.value) {
                return Err(BackendContractError::InvalidBlockArguments(
                    "block parameters contain a duplicate value identifier",
                ));
            }
        }
    }
    if !ids.contains(&entry) {
        return Err(BackendContractError::MissingEntryBlock);
    }
    Ok(ids)
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
        Role::Ins => {
            matches!(&parameter.passing, BackendPassingMode::ExclusiveLoan { .. })
                || matches!(
                    &parameter.passing,
                    BackendPassingMode::ExclusiveScalarLoan { ty } if *ty == parameter.ty
                )
        }
        Role::Dat => {
            matches!(&parameter.passing, BackendPassingMode::OwnershipTransfer { .. })
                || (matches!(&parameter.passing, BackendPassingMode::Direct)
                    && is_direct_role_value(parameter.ty))
        }
    };
    valid.then_some(()).ok_or(BackendContractError::RolePassingMismatch)
}

fn is_direct_role_value(ty: BackendValueType) -> bool {
    matches!(
        ty,
        BackendValueType::Unit
            | BackendValueType::Bool
            | BackendValueType::Integer { .. }
            | BackendValueType::Float { .. }
    )
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

pub(super) fn validate_value_type(ty: BackendValueType) -> Result<(), BackendContractError> {
    match ty {
        BackendValueType::Integer { width: 0, .. } | BackendValueType::Float { width: 0 } => {
            Err(BackendContractError::InvalidValueType)
        }
        BackendValueType::Aggregate { layout }
        | BackendValueType::Region { layout }
        | BackendValueType::NichePointerOption { layout, .. } => layout.validate(),
        BackendValueType::Enum { layout } => layout.validate(),
        BackendValueType::Unit
        | BackendValueType::Bool
        | BackendValueType::Integer { .. }
        | BackendValueType::Float { .. }
        | BackendValueType::Pointer => Ok(()),
    }
}
