use cranelift_frontend::FunctionBuilder;

use crate::ast::Block;
use crate::lexer::SourceSpan;
use crate::semantic::LoopExitKind;

use super::model::{NativeCleanupPlan, NativeCleanupSchedule, NativeInstruction};
use super::native::NativeEmitError;

pub(super) fn emit_return_cleanup(
    function: &mut FunctionBuilder<'_>,
    schedule: &NativeCleanupSchedule,
    span: SourceSpan,
) -> Result<(), NativeEmitError> {
    let plan = schedule.return_plan(span).ok_or_else(|| {
        NativeEmitError(format!("missing native return cleanup plan at {span:?}"))
    })?;
    for scope in &plan.scopes {
        emit_scope_instructions(function, scope);
    }
    Ok(())
}

pub(super) fn emit_loop_cleanup(
    function: &mut FunctionBuilder<'_>,
    schedule: &NativeCleanupSchedule,
    span: SourceSpan,
    kind: LoopExitKind,
) -> Result<(), NativeEmitError> {
    let plan = schedule
        .loop_plan(span, &kind)
        .ok_or_else(|| NativeEmitError(format!("missing native loop cleanup plan at {span:?}")))?;
    for scope in &plan.scopes {
        emit_scope_instructions(function, scope);
    }
    Ok(())
}

pub(super) fn emit_scope_cleanup(
    function: &mut FunctionBuilder<'_>,
    schedule: &NativeCleanupSchedule,
    block: &Block,
) -> Result<(), NativeEmitError> {
    let plan = schedule.scope(block.span).ok_or_else(|| {
        NativeEmitError(format!("missing native scope cleanup plan at {:?}", block.span))
    })?;
    emit_scope_instructions(function, plan);
    Ok(())
}

fn emit_scope_instructions(_function: &mut FunctionBuilder<'_>, plan: &NativeCleanupPlan) {
    for instruction in &plan.instructions {
        match instruction {
            NativeInstruction::EndBorrow { .. } | NativeInstruction::DropBinding { .. } => {}
        }
    }
}
