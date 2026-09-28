use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData};
use cranelift_frontend::FunctionBuilder;

use crate::lexer::SourceSpan;
use crate::semantic::LoopExitKind;

use super::layout::LayoutRegistry;
use super::model::{NativeCleanupPlan, NativeCleanupSchedule, NativeInstruction};
use super::native::FunctionRef;
use super::native::NativeEmitError;
use super::structs::emit_binding_drop;
use super::types::NativeType;

pub(super) fn emit_return_cleanup(
    function: &mut FunctionBuilder<'_>,
    schedule: &NativeCleanupSchedule,
    span: SourceSpan,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    let plan = schedule.return_plan(span).ok_or_else(|| {
        NativeEmitError(format!("missing native return cleanup plan at {span:?}"))
    })?;
    for scope in &plan.scopes {
        emit_scope_instructions(function, scope, locals, types, functions, layouts)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn emit_loop_cleanup(
    function: &mut FunctionBuilder<'_>,
    schedule: &NativeCleanupSchedule,
    span: SourceSpan,
    kind: LoopExitKind,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    let plan = schedule
        .loop_plan(span, &kind)
        .ok_or_else(|| NativeEmitError(format!("missing native loop cleanup plan at {span:?}")))?;
    for scope in &plan.scopes {
        emit_scope_instructions(function, scope, locals, types, functions, layouts)?;
    }
    Ok(())
}

pub(super) fn emit_scope_cleanup(
    function: &mut FunctionBuilder<'_>,
    schedule: &NativeCleanupSchedule,
    span: SourceSpan,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    let plan = schedule.scope(span).ok_or_else(|| {
        NativeEmitError(format!("missing native scope cleanup plan at {:?}", span))
    })?;
    emit_scope_instructions(function, plan, locals, types, functions, layouts)?;
    Ok(())
}

fn emit_scope_instructions(
    function: &mut FunctionBuilder<'_>,
    plan: &NativeCleanupPlan,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    for instruction in &plan.instructions {
        emit_cleanup_instruction(function, instruction, locals, types, functions, layouts)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn emit_cleanup_instruction(
    function: &mut FunctionBuilder<'_>,
    instruction: &NativeInstruction,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    match instruction {
        NativeInstruction::EndBorrow { .. } => Ok(()),
        NativeInstruction::DropPayloadField { .. } => {
            emit_payload_cleanup(function, instruction, locals, types, functions, layouts)
        }
        NativeInstruction::DropBinding { name, .. } => {
            emit_binding_drop(function, name, locals, types, functions, layouts)
        }
        NativeInstruction::DropBindingFields { .. } => {
            emit_partial_binding_cleanup(function, instruction, locals, types, functions, layouts)
        }
        NativeInstruction::ResetArena { name, .. } => {
            reset_arena(function, name, locals, types, layouts)
        }
    }
}

fn emit_payload_cleanup(
    function: &mut FunctionBuilder<'_>,
    instruction: &NativeInstruction,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    let NativeInstruction::DropPayloadField { name, enum_name, variant, field, .. } = instruction
    else {
        return Ok(());
    };
    super::enums::emit_enum_payload_drop(
        function, name, enum_name, variant, field, locals, types, functions, layouts,
    )
}

fn emit_partial_binding_cleanup(
    function: &mut FunctionBuilder<'_>,
    instruction: &NativeInstruction,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    let NativeInstruction::DropBindingFields { name, moved_fields, .. } = instruction else {
        return Ok(());
    };
    super::structs::emit_partial_binding_drop(
        function,
        name,
        moved_fields,
        locals,
        types,
        functions,
        layouts,
    )
}

fn reset_arena(
    function: &mut FunctionBuilder<'_>,
    name: &str,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    let Some(NativeType::Arena(capacity)) =
        types.iter().find(|(binding, _)| binding.as_str() == name).map(|(_, ty)| *ty)
    else {
        return Err(NativeEmitError(format!("arena binding `{name}` is unavailable")));
    };
    let address = locals
        .iter()
        .find(|(binding, _)| binding.as_str() == name)
        .map(|(_, value)| *value)
        .ok_or_else(|| NativeEmitError(format!("arena binding `{name}` is unavailable")))?;
    let offset_address = function.ins().iadd_imm_s(address, i64::from(capacity));
    let zero = function.ins().iconst(layouts.pointer_type, 0);
    function.ins().store(MemFlagsData::new(), zero, offset_address, 0);
    Ok(())
}
