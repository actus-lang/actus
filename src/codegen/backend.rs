use std::fmt::{Display, Formatter};

use serde::Deserialize;

use crate::target::TargetSpec;

/// Integer operations of the backend contract. `Add`, `Subtract` and
/// `Multiply` wrap modulo 2^N; the `Trapping` forms trap when the result does
/// not fit and the `Saturating` forms clamp it to the type's bounds
/// (ADR-0115).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendIntegerBinaryOp {
    Add,
    Subtract,
    Multiply,
    TrappingAdd,
    TrappingSubtract,
    TrappingMultiply,
    SaturatingAdd,
    SaturatingSubtract,
    SaturatingMultiply,
    Divide,
    Remainder,
    ShiftLeft,
    ShiftRight,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    LogicalAnd,
    LogicalOr,
}

impl BackendIntegerBinaryOp {
    /// The operation a source operator lowers to; `None` for comparisons,
    /// which lower to compare instructions.
    pub const fn from_source(operator: crate::ast::BinaryOp) -> Option<Self> {
        use crate::ast::BinaryOp;
        Some(match operator {
            BinaryOp::Add => Self::TrappingAdd,
            BinaryOp::Subtract => Self::TrappingSubtract,
            BinaryOp::Multiply => Self::TrappingMultiply,
            BinaryOp::WrappingAdd => Self::Add,
            BinaryOp::WrappingSubtract => Self::Subtract,
            BinaryOp::WrappingMultiply => Self::Multiply,
            BinaryOp::SaturatingAdd => Self::SaturatingAdd,
            BinaryOp::SaturatingSubtract => Self::SaturatingSubtract,
            BinaryOp::SaturatingMultiply => Self::SaturatingMultiply,
            BinaryOp::Divide => Self::Divide,
            BinaryOp::Remainder => Self::Remainder,
            BinaryOp::ShiftLeft => Self::ShiftLeft,
            BinaryOp::ShiftRight => Self::ShiftRight,
            BinaryOp::BitwiseAnd => Self::BitwiseAnd,
            BinaryOp::BitwiseOr => Self::BitwiseOr,
            BinaryOp::BitwiseXor => Self::BitwiseXor,
            BinaryOp::LogicalAnd => Self::LogicalAnd,
            BinaryOp::LogicalOr => Self::LogicalOr,
            BinaryOp::LessThan
            | BinaryOp::LessEquals
            | BinaryOp::GreaterThan
            | BinaryOp::GreaterEquals
            | BinaryOp::Equals
            | BinaryOp::NotEquals => return None,
        })
    }

    /// The operation `target op= value` applies.
    pub const fn from_compound(operator: crate::ast::CompoundAssignmentOp) -> Self {
        use crate::ast::CompoundAssignmentOp;
        match operator {
            CompoundAssignmentOp::Add => Self::TrappingAdd,
            CompoundAssignmentOp::Subtract => Self::TrappingSubtract,
            CompoundAssignmentOp::Multiply => Self::TrappingMultiply,
            CompoundAssignmentOp::WrappingAdd => Self::Add,
            CompoundAssignmentOp::WrappingSubtract => Self::Subtract,
            CompoundAssignmentOp::WrappingMultiply => Self::Multiply,
            CompoundAssignmentOp::SaturatingAdd => Self::SaturatingAdd,
            CompoundAssignmentOp::SaturatingSubtract => Self::SaturatingSubtract,
            CompoundAssignmentOp::SaturatingMultiply => Self::SaturatingMultiply,
            CompoundAssignmentOp::Divide => Self::Divide,
            CompoundAssignmentOp::Remainder => Self::Remainder,
            CompoundAssignmentOp::ShiftLeft => Self::ShiftLeft,
            CompoundAssignmentOp::ShiftRight => Self::ShiftRight,
            CompoundAssignmentOp::BitwiseAnd => Self::BitwiseAnd,
            CompoundAssignmentOp::BitwiseOr => Self::BitwiseOr,
            CompoundAssignmentOp::BitwiseXor => Self::BitwiseXor,
        }
    }
}

/// `Negate` wraps (`-%`); `TrappingNegate` traps on `-MIN` and on any
/// nonzero unsigned operand (ADR-0115).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendIntegerUnaryOp {
    Negate,
    TrappingNegate,
    LogicalNot,
    BitwiseNot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendIntegerCompareOp {
    LessThan,
    LessEquals,
    GreaterThan,
    GreaterEquals,
    Equals,
    NotEquals,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BackendKind {
    Cranelift,
    Llvm,
}

// LLVM is the default native backend. Cranelift remains selectable explicitly
// through the feature set or the manifest backend setting.
#[allow(clippy::derivable_impls)]
impl Default for BackendKind {
    fn default() -> Self {
        #[cfg(feature = "llvm")]
        {
            Self::Llvm
        }

        #[cfg(all(not(feature = "llvm"), feature = "cranelift"))]
        {
            Self::Cranelift
        }

        #[cfg(not(any(feature = "cranelift", feature = "llvm")))]
        {
            Self::Cranelift
        }
    }
}

impl BackendKind {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Cranelift => "cranelift",
            Self::Llvm => "llvm",
        }
    }

    pub fn validate_target(self, target: &TargetSpec) -> Result<(), BackendTargetError> {
        match self {
            Self::Cranelift if cfg!(feature = "cranelift") => Ok(()),
            Self::Cranelift => Err(BackendTargetError::Unavailable {
                backend: self,
                target: target.triple().to_string(),
            }),
            Self::Llvm if cfg!(feature = "llvm") => Ok(()),
            Self::Llvm => Err(BackendTargetError::Unavailable {
                backend: self,
                target: target.triple().to_string(),
            }),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendTargetError {
    Unavailable { backend: BackendKind, target: String },
}

impl Display for BackendTargetError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable { backend, target } => write!(
                formatter,
                "backend `{}` is not available for target `{target}`",
                backend.name()
            ),
        }
    }
}

impl std::error::Error for BackendTargetError {}

#[cfg(test)]
mod tests {
    use super::{BackendKind, BackendTargetError};
    use crate::target::TargetSpec;

    #[test]
    fn backend_selection_is_independent_from_target_selection() {
        let target = TargetSpec::parse("x86_64-unknown-linux-gnu").expect("target should parse");
        let cranelift_result = BackendKind::Cranelift.validate_target(&target);
        if cfg!(feature = "cranelift") {
            assert_eq!(cranelift_result, Ok(()));
        } else {
            assert!(matches!(cranelift_result, Err(BackendTargetError::Unavailable { .. })));
        }
        assert_eq!(BackendKind::Llvm.name(), "llvm");
    }

    #[test]
    fn unavailable_backend_target_pairs_fail_with_typed_diagnostics() {
        let target = TargetSpec::parse("x86_64-unknown-linux-gnu").expect("target should parse");
        let result = BackendKind::Llvm.validate_target(&target);
        if cfg!(feature = "llvm") {
            assert_eq!(result, Ok(()));
        } else {
            assert_eq!(
                result,
                Err(BackendTargetError::Unavailable {
                    backend: BackendKind::Llvm,
                    target: "x86_64-unknown-linux-gnu".to_owned(),
                })
            );
        }
    }
}
