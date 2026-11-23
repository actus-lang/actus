//! How native code reacts to a failed runtime check (ADR-0117).

use std::collections::BTreeMap;

use super::{BackendInstruction, BackendIntegerBinaryOp, BackendIntegerUnaryOp};
use crate::lexer::SourceSpan;

/// What a failed runtime check, such as an integer overflow or an index out
/// of bounds, does in the emitted code.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum RuntimeCheckReporting {
    /// The check executes the target's trap instruction. Freestanding
    /// objects and libraries use this: they may run without the runtime.
    #[default]
    Trap,
    /// The check calls the runtime's check-failure report,
    /// [`crate::runtime::RUNTIME_CHECK_FAILED_SYMBOL`], with a message that
    /// names the check and where it is in the source. `locations` maps the
    /// span of a checking instruction to its `path:line:column`.
    Report { locations: BTreeMap<(usize, usize), String> },
}

impl RuntimeCheckReporting {
    /// The message a failed check at `span` reports, or `None` when the
    /// check traps. `reason` names the check, such as `integer overflow`.
    pub fn message(&self, reason: &str, span: SourceSpan) -> Option<String> {
        let Self::Report { locations } = self else { return None };
        Some(match locations.get(&(span.start, span.end)) {
            Some(location) => format!("{reason} at {location}"),
            None => reason.to_owned(),
        })
    }

    /// The message a failed runtime check of `instruction` reports, or `None`
    /// when the check traps or the instruction performs no check.
    pub fn failure(&self, instruction: &BackendInstruction) -> Option<String> {
        let (reason, span) = runtime_check(instruction)?;
        self.message(reason, span)
    }
}

/// What `instruction` checks at run time and where, or `None` when it
/// cannot fail a runtime check.
pub fn runtime_check(instruction: &BackendInstruction) -> Option<(&str, SourceSpan)> {
    match instruction {
        BackendInstruction::IntegerBinary { operation, span, .. } => matches!(
            operation,
            BackendIntegerBinaryOp::TrappingAdd
                | BackendIntegerBinaryOp::TrappingSubtract
                | BackendIntegerBinaryOp::TrappingMultiply
        )
        .then_some(("integer overflow", *span)),
        BackendInstruction::IntegerUnary { operation, span, .. } => {
            matches!(operation, BackendIntegerUnaryOp::TrappingNegate)
                .then_some(("integer overflow", *span))
        }
        BackendInstruction::TrapIfSignedDivisionOverflow { span, .. } => {
            Some(("integer overflow", *span))
        }
        BackendInstruction::IntegerCast { span, .. } => Some(("integer value out of range", *span)),
        BackendInstruction::TrapIfZero { span, .. } => Some(("division by zero", *span)),
        BackendInstruction::TrapIfShiftCountOutOfRange { span, .. } => {
            Some(("shift count out of range", *span))
        }
        BackendInstruction::TrapIfArrayIndexOutOfBounds { span, .. } => {
            Some(("index out of bounds", *span))
        }
        BackendInstruction::TrapIfBufferIndexOutOfBounds { span, .. } => {
            Some(("index out of bounds", *span))
        }
        BackendInstruction::ArenaPlace { span, .. } => Some(("arena capacity exceeded", *span)),
        BackendInstruction::Trap { reason, span } => Some((reason, *span)),
        _ => None,
    }
}
