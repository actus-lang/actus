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
            validate_parameter(parameter)?;
        }
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendContractError {
    InvalidLayout(&'static str),
    InvalidFunctionSymbol,
    MissingEntryBlock,
    EmptyParameterName,
    DuplicateParameterName,
    DuplicateBlockId,
    InvalidBlockReference,
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

#[cfg(test)]
mod diagnostic_tests {
    use super::*;

    #[test]
    fn reports_unsupported_operations_with_target_context() {
        let error = BackendContractError::Unsupported(BackendUnsupportedOperation::new(
            "aggregate-return",
            SourceSpan::new(12, 20),
            BackendTargetContext {
                backend: "llvm".to_owned(),
                target: "thumbv7em-none-eabihf".to_owned(),
            },
            "aggregate return lowering is not implemented",
        ));
        let rendered = error.to_string();
        assert!(rendered.contains("aggregate-return"));
        assert!(rendered.contains("thumbv7em-none-eabihf"));
        assert!(rendered.contains("12"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verb_symbol() -> SymbolIdentity {
        SymbolIdentity::new("test", SymbolKind::Verb, "run").expect("valid verb symbol")
    }

    #[test]
    fn validates_aligned_aggregate_layouts() {
        assert!(BackendLayout { size: 16, alignment: 8 }.validate().is_ok());
        assert!(BackendLayout { size: 12, alignment: 8 }.validate().is_err());
        assert!(BackendLayout { size: 16, alignment: 3 }.validate().is_err());
    }

    #[test]
    fn preserves_role_metadata_without_a_machine_backend() {
        let function = BackendFunction {
            symbol: verb_symbol(),
            parameters: vec![BackendParameter {
                name: "input".to_owned(),
                role: Role::Abs,
                ty: BackendValueType::Integer { signed: false, width: 32 },
                passing: BackendPassingMode::Borrowed {
                    layout: BackendLayout { size: 4, alignment: 4 },
                },
            }],
            return_value: BackendReturn {
                ty: BackendValueType::Unit,
                passing: BackendReturnMode::Direct,
            },
            entry: BackendBlockId(0),
            blocks: vec![BackendBlock {
                id: BackendBlockId(0),
                instructions: vec![BackendInstruction::Return {
                    value: None,
                    span: SourceSpan::new(0, 1),
                }],
            }],
        };

        assert_eq!(function.parameters[0].role, Role::Abs);
        assert!(function.validate().is_ok());
    }

    #[test]
    fn rejects_role_and_passing_mode_mismatches() {
        let parameter = BackendParameter {
            name: "loan".to_owned(),
            role: Role::Ins,
            ty: BackendValueType::Pointer,
            passing: BackendPassingMode::Borrowed {
                layout: BackendLayout { size: 8, alignment: 8 },
            },
        };
        assert_eq!(validate_parameter(&parameter), Err(BackendContractError::RolePassingMismatch));
    }

    #[test]
    fn validates_aggregate_return_slots_without_a_backend() {
        let layout = BackendLayout { size: 16, alignment: 8 };
        let function = BackendFunction {
            symbol: verb_symbol(),
            parameters: Vec::new(),
            return_value: BackendReturn {
                ty: BackendValueType::Aggregate { layout },
                passing: BackendReturnMode::ReturnSlot { layout },
            },
            entry: BackendBlockId(0),
            blocks: vec![BackendBlock { id: BackendBlockId(0), instructions: Vec::new() }],
        };
        assert!(function.validate().is_ok());
    }

    #[test]
    fn rejects_return_slots_for_non_aggregate_values() {
        let function = BackendFunction {
            symbol: verb_symbol(),
            parameters: Vec::new(),
            return_value: BackendReturn {
                ty: BackendValueType::Integer { signed: false, width: 32 },
                passing: BackendReturnMode::ReturnSlot {
                    layout: BackendLayout { size: 4, alignment: 4 },
                },
            },
            entry: BackendBlockId(0),
            blocks: vec![BackendBlock { id: BackendBlockId(0), instructions: Vec::new() }],
        };
        assert_eq!(function.validate(), Err(BackendContractError::ReturnSlotTypeMismatch));
    }

    #[test]
    fn rejects_duplicate_parameters_and_blocks() {
        let function = BackendFunction {
            symbol: verb_symbol(),
            parameters: vec![
                BackendParameter {
                    name: "value".to_owned(),
                    role: Role::Erg,
                    ty: BackendValueType::Integer { signed: false, width: 32 },
                    passing: BackendPassingMode::Direct,
                },
                BackendParameter {
                    name: "value".to_owned(),
                    role: Role::Erg,
                    ty: BackendValueType::Integer { signed: false, width: 32 },
                    passing: BackendPassingMode::Direct,
                },
            ],
            return_value: BackendReturn {
                ty: BackendValueType::Unit,
                passing: BackendReturnMode::Direct,
            },
            entry: BackendBlockId(0),
            blocks: vec![
                BackendBlock { id: BackendBlockId(0), instructions: Vec::new() },
                BackendBlock { id: BackendBlockId(1), instructions: Vec::new() },
            ],
        };
        assert_eq!(function.validate(), Err(BackendContractError::DuplicateParameterName));

        let mut blocks_only = function;
        blocks_only.parameters.clear();
        blocks_only.blocks[1].id = BackendBlockId(0);
        assert_eq!(blocks_only.validate(), Err(BackendContractError::DuplicateBlockId));
    }

    #[test]
    fn validates_control_flow_targets_without_a_machine_backend() {
        let valid = BackendFunction {
            symbol: verb_symbol(),
            parameters: Vec::new(),
            return_value: BackendReturn {
                ty: BackendValueType::Unit,
                passing: BackendReturnMode::Direct,
            },
            entry: BackendBlockId(0),
            blocks: vec![
                BackendBlock {
                    id: BackendBlockId(0),
                    instructions: vec![BackendInstruction::Jump {
                        target: BackendBlockId(1),
                        span: SourceSpan::new(0, 1),
                    }],
                },
                BackendBlock { id: BackendBlockId(1), instructions: Vec::new() },
            ],
        };
        assert!(valid.validate().is_ok());

        let mut invalid = valid;
        invalid.blocks[0].instructions[0] =
            BackendInstruction::Jump { target: BackendBlockId(2), span: SourceSpan::new(0, 1) };
        assert_eq!(invalid.validate(), Err(BackendContractError::InvalidBlockReference));
    }
}
