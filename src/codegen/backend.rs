use std::fmt::{Display, Formatter};

use serde::Deserialize;

use crate::target::TargetSpec;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendIntegerBinaryOp {
    Add,
    Subtract,
    Multiply,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BackendKind {
    Cranelift,
    Llvm,
}

impl Default for BackendKind {
    fn default() -> Self {
        #[cfg(feature = "cranelift")]
        {
            return Self::Cranelift;
        }

        #[cfg(all(not(feature = "cranelift"), feature = "llvm"))]
        {
            return Self::Llvm;
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
        assert_eq!(BackendKind::Cranelift.validate_target(&target), Ok(()));
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
