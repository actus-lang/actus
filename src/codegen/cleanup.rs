use std::collections::HashMap;

use cranelift_frontend::FunctionBuilder;

use crate::ast::Block;
use crate::lexer::SourceSpan;
use crate::semantic::LoopExitKind;

use super::expressions::emit_buffer_drop;
use super::model::{NativeCleanupPlan, NativeCleanupSchedule, NativeInstruction};
use super::native::FunctionRef;
use super::native::NativeEmitError;
use super::types::NativeType;

pub(super) fn emit_return_cleanup(
    function: &mut FunctionBuilder<'_>,
    schedule: &NativeCleanupSchedule,
    span: SourceSpan,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
) -> Result<(), NativeEmitError> {
    let plan = schedule.return_plan(span).ok_or_else(|| {
        NativeEmitError(format!("missing native return cleanup plan at {span:?}"))
    })?;
    for scope in &plan.scopes {
        emit_scope_instructions(function, scope, locals, types, functions)?;
    }
    Ok(())
}

pub(super) fn emit_loop_cleanup(
    function: &mut FunctionBuilder<'_>,
    schedule: &NativeCleanupSchedule,
    span: SourceSpan,
    kind: LoopExitKind,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
) -> Result<(), NativeEmitError> {
    let plan = schedule
        .loop_plan(span, &kind)
        .ok_or_else(|| NativeEmitError(format!("missing native loop cleanup plan at {span:?}")))?;
    for scope in &plan.scopes {
        emit_scope_instructions(function, scope, locals, types, functions)?;
    }
    Ok(())
}

pub(super) fn emit_scope_cleanup(
    function: &mut FunctionBuilder<'_>,
    schedule: &NativeCleanupSchedule,
    block: &Block,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
) -> Result<(), NativeEmitError> {
    let plan = schedule.scope(block.span).ok_or_else(|| {
        NativeEmitError(format!("missing native scope cleanup plan at {:?}", block.span))
    })?;
    emit_scope_instructions(function, plan, locals, types, functions)?;
    Ok(())
}

fn emit_scope_instructions(
    function: &mut FunctionBuilder<'_>,
    plan: &NativeCleanupPlan,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
) -> Result<(), NativeEmitError> {
    for instruction in &plan.instructions {
        match instruction {
            NativeInstruction::EndBorrow { .. } => {}
            NativeInstruction::DropBinding { name, .. }
                if types.get(name) == Some(&NativeType::Buffer) =>
            {
                emit_buffer_drop(function, name, locals, functions)?;
            }
            NativeInstruction::DropBinding { .. } => {}
        }
    }
    Ok(())
}
