use crate::ast::Role;
use crate::lexer::SourceSpan;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackendValueId(pub u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackendBlockId(pub u32);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendInstruction {
    Call {
        callee: SymbolIdentity,
        arguments: Vec<BackendValueId>,
        result: Option<BackendValueId>,
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
        if self.blocks.iter().filter(|block| block.id == self.entry).count() != 1 {
            return Err(BackendContractError::MissingEntryBlock);
        }
        for parameter in &self.parameters {
            validate_parameter(parameter)?;
        }
        if let BackendReturnMode::ReturnSlot { layout } = self.return_value.passing {
            layout.validate()?;
        }
        for block in &self.blocks {
            for instruction in &block.instructions {
                if let BackendInstruction::Call { callee, .. } = instruction {
                    if callee.as_str().is_empty() {
                        return Err(BackendContractError::InvalidCallSymbol);
                    }
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendContractError {
    InvalidLayout(&'static str),
    InvalidFunctionSymbol,
    MissingEntryBlock,
    InvalidCallSymbol,
    RolePassingMismatch,
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
}
