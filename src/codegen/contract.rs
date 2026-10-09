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
    pub return_type: BackendValueType,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendContractError {
    InvalidLayout(&'static str),
    InvalidFunctionSymbol,
    MissingEntryBlock,
    InvalidCallSymbol,
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
            }],
            return_type: BackendValueType::Unit,
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
}
