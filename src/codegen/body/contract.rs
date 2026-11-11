use crate::ast::Stmt;
use crate::lexer::SourceSpan;

use super::super::{BackendContractError, BackendInstruction, BackendTargetContext};

pub(in crate::codegen) fn terminates(instruction: Option<&BackendInstruction>) -> bool {
    matches!(
        instruction,
        Some(BackendInstruction::Branch { .. })
            | Some(BackendInstruction::Jump { .. })
            | Some(BackendInstruction::Fail { .. })
            | Some(BackendInstruction::Return { .. })
            | Some(BackendInstruction::Trap { .. })
    )
}

pub(in crate::codegen) fn terminates_without_continuation(
    instruction: Option<&BackendInstruction>,
    continuation: super::super::BackendBlockId,
) -> bool {
    match instruction {
        Some(BackendInstruction::Jump { target, .. }) if *target == continuation => false,
        other => terminates(other),
    }
}

pub(in crate::codegen) fn ensure_jump(
    mut block: super::super::BackendBlock,
    target: super::super::BackendBlockId,
    span: SourceSpan,
) -> super::super::BackendBlock {
    if !terminates(block.instructions.last()) {
        block.instructions.push(BackendInstruction::Jump { target, arguments: Vec::new(), span });
    }
    block
}

pub(in crate::codegen) fn statement_span(statement: &Stmt) -> SourceSpan {
    match statement {
        Stmt::OwnerDecl { span, .. }
        | Stmt::Assignment { span, .. }
        | Stmt::CompoundAssignment { span, .. }
        | Stmt::Expression { span, .. }
        | Stmt::If { span, .. }
        | Stmt::Return { span, .. }
        | Stmt::ForRange { span, .. }
        | Stmt::ForArray { span, .. }
        | Stmt::Break { span }
        | Stmt::Continue { span }
        | Stmt::Drop { span, .. } => *span,
        Stmt::Loop(block) | Stmt::Block(block) => block.span,
    }
}

pub(in crate::codegen) fn unsupported_name(statement: &Stmt) -> &'static str {
    match statement {
        Stmt::OwnerDecl { .. } => "owner declaration",
        Stmt::Assignment { .. } => "assignment",
        Stmt::CompoundAssignment { .. } => "compound assignment",
        Stmt::Expression { .. } => "expression",
        Stmt::If { .. } => "if statement",
        Stmt::Return { .. } => "return statement",
        Stmt::Loop(_) => "loop",
        Stmt::ForRange { .. } => "range loop",
        Stmt::ForArray { .. } => "array loop",
        Stmt::Break { .. } => "break",
        Stmt::Continue { .. } => "continue",
        Stmt::Drop { .. } => "drop statement",
        Stmt::Block(_) => "nested block",
    }
}

pub(in crate::codegen) fn unsupported(
    span: SourceSpan,
    name: impl Into<String>,
) -> BackendContractError {
    BackendContractError::Unsupported(super::super::BackendUnsupportedOperation::new(
        "body-lowering",
        span,
        BackendTargetContext {
            backend: "neutral-contract".to_owned(),
            target: "semantic-model".to_owned(),
        },
        format!("{} has no neutral lowering contract yet", name.into()),
    ))
}
