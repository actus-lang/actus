use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData};
use cranelift_frontend::FunctionBuilder;

use crate::ast::Expr;
use crate::lexer::SourceSpan;
use crate::semantic::LoopExitKind;

use super::super::contract::BackendCleanupResolvedDescriptor;
use super::super::model::{LoweredCleanupInstruction, LoweredCleanupPlan, LoweredCleanupSchedule};
use super::layout::LayoutRegistry;
use super::native::{BackendEmitError, FunctionRef};
use super::structs::emit_binding_drop;
use super::types::NativeType;

#[allow(clippy::too_many_arguments)]
pub(super) fn emit_return_cleanup(
    function: &mut FunctionBuilder<'_>,
    schedule: &LoweredCleanupSchedule,
    span: SourceSpan,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
    returned_expression: Option<&Expr>,
) -> Result<(), BackendEmitError> {
    let plan = schedule.return_plan(span).ok_or_else(|| {
        BackendEmitError(format!("missing native return cleanup plan at {span:?}"))
    })?;
    let returned_binding = returned_expression.and_then(returned_binding_name);
    for scope in &plan.scopes {
        emit_scope_instructions_filtered(
            function,
            scope,
            locals,
            types,
            functions,
            layouts,
            returned_binding,
            schedule.resolved_plan(scope),
        )?;
    }
    Ok(())
}

fn returned_binding_name(expression: &Expr) -> Option<&str> {
    match expression {
        Expr::Identifier { name, .. } => Some(name),
        Expr::Grouping { expression, .. } => returned_binding_name(expression),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn emit_loop_cleanup(
    function: &mut FunctionBuilder<'_>,
    schedule: &LoweredCleanupSchedule,
    span: SourceSpan,
    kind: LoopExitKind,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<(), BackendEmitError> {
    let plan = schedule
        .loop_plan(span, &kind)
        .ok_or_else(|| BackendEmitError(format!("missing native loop cleanup plan at {span:?}")))?;
    for scope in &plan.scopes {
        emit_scope_instructions(
            function,
            scope,
            locals,
            types,
            functions,
            layouts,
            schedule.resolved_plan(scope),
        )?;
    }
    Ok(())
}

pub(super) fn emit_scope_cleanup(
    function: &mut FunctionBuilder<'_>,
    schedule: &LoweredCleanupSchedule,
    span: SourceSpan,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<(), BackendEmitError> {
    let plan = schedule.scope(span).ok_or_else(|| {
        BackendEmitError(format!("missing native scope cleanup plan at {:?}", span))
    })?;
    emit_scope_instructions(
        function,
        plan,
        locals,
        types,
        functions,
        layouts,
        schedule.resolved_plan(plan),
    )?;
    Ok(())
}

fn emit_scope_instructions(
    function: &mut FunctionBuilder<'_>,
    plan: &LoweredCleanupPlan,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
    resolved: Option<&super::super::contract::BackendCleanupPlan>,
) -> Result<(), BackendEmitError> {
    for (index, instruction) in plan.instructions.iter().enumerate() {
        emit_cleanup_instruction(
            function,
            instruction,
            locals,
            types,
            functions,
            layouts,
            resolved.and_then(|plan| plan.resolved_bridges.get(index).map(Vec::as_slice)),
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn emit_scope_instructions_filtered(
    function: &mut FunctionBuilder<'_>,
    plan: &LoweredCleanupPlan,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
    returned_binding: Option<&str>,
    resolved: Option<&super::super::contract::BackendCleanupPlan>,
) -> Result<(), BackendEmitError> {
    for (index, instruction) in plan.instructions.iter().enumerate() {
        if matches!(
            instruction,
            LoweredCleanupInstruction::DropBinding { name, .. }
                if Some(name.as_str()) == returned_binding
        ) {
            continue;
        }
        emit_cleanup_instruction(
            function,
            instruction,
            locals,
            types,
            functions,
            layouts,
            resolved.and_then(|plan| plan.resolved_bridges.get(index).map(Vec::as_slice)),
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn emit_cleanup_instruction(
    function: &mut FunctionBuilder<'_>,
    instruction: &LoweredCleanupInstruction,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
    resolved: Option<&[BackendCleanupResolvedDescriptor]>,
) -> Result<(), BackendEmitError> {
    match instruction {
        LoweredCleanupInstruction::EndBorrow { .. } => Ok(()),
        LoweredCleanupInstruction::DropPayloadField { .. } => {
            emit_payload_cleanup(function, instruction, locals, types, functions, layouts)
        }
        LoweredCleanupInstruction::DropBinding { name, .. } => {
            if let Some(descriptor) = resolved.and_then(|descriptors| descriptors.first()) {
                emit_resolved_binding_drop(function, name, descriptor, locals, functions, layouts)?;
            } else {
                emit_binding_drop(function, name, locals, types, functions, layouts)?;
            }
            Ok(())
        }
        LoweredCleanupInstruction::DropBindingFields { .. } => {
            emit_partial_binding_cleanup(function, instruction, locals, types, functions, layouts)
        }
        LoweredCleanupInstruction::ResetArena { name, .. } => {
            reset_arena(function, name, locals, types, layouts)
        }
    }
}

fn emit_resolved_binding_drop(
    function: &mut FunctionBuilder<'_>,
    name: &str,
    descriptor: &BackendCleanupResolvedDescriptor,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<(), BackendEmitError> {
    if matches!(
        descriptor.descriptor.argument,
        super::super::contract::BackendCleanupArgument::FieldAddress
    ) || descriptor.field.is_some()
    {
        return Err(BackendEmitError(
            "direct cleanup descriptor unexpectedly contains a field address".to_owned(),
        ));
    }
    let value = locals
        .iter()
        .find(|(binding, _)| binding.as_str() == name)
        .map(|(_, value)| *value)
        .ok_or_else(|| BackendEmitError(format!("native binding `{name}` is unavailable")))?;
    let symbol = match descriptor.descriptor.bridge {
        super::super::contract::BackendCleanupBridge::BufferDrop => {
            crate::runtime::BUFFER_DROP_SYMBOL.to_owned()
        }
        super::super::contract::BackendCleanupBridge::RegionDrop => {
            super::performance::dispatch_key(NativeType::Region, "drop")
        }
        super::super::contract::BackendCleanupBridge::EnumDrop { .. } => {
            crate::runtime::ENUM_DROP_SYMBOL.to_owned()
        }
        super::super::contract::BackendCleanupBridge::AggregateDrop { .. } => {
            crate::runtime::AGGREGATE_DROP_SYMBOL.to_owned()
        }
        super::super::contract::BackendCleanupBridge::PerformanceDrop => {
            return Err(BackendEmitError(
                "Cranelift user-defined Drop performance emission is not implemented".to_owned(),
            ));
        }
    };
    let target = functions.get(&symbol).ok_or_else(|| {
        BackendEmitError(format!("native runtime function `{symbol}` is unavailable"))
    })?;
    let mut arguments = vec![value];
    if let super::super::contract::BackendCleanupBridge::EnumDrop { .. } =
        descriptor.descriptor.bridge
    {
        arguments
            .push(function.ins().iconst(layouts.pointer_type, i64::from(descriptor.layout.size)));
    } else if let super::super::contract::BackendCleanupBridge::AggregateDrop { .. } =
        descriptor.descriptor.bridge
    {
        arguments
            .push(function.ins().iconst(layouts.pointer_type, i64::from(descriptor.layout.size)));
        arguments.push(
            function.ins().iconst(layouts.pointer_type, i64::from(descriptor.layout.alignment)),
        );
    }
    function.ins().call(target.reference, &arguments);
    Ok(())
}

fn emit_payload_cleanup(
    function: &mut FunctionBuilder<'_>,
    instruction: &LoweredCleanupInstruction,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<(), BackendEmitError> {
    let LoweredCleanupInstruction::DropPayloadField { name, enum_name, variant, field, .. } =
        instruction
    else {
        return Ok(());
    };
    super::enums::emit_enum_payload_drop(
        function, name, enum_name, variant, field, locals, types, functions, layouts,
    )
}

fn emit_partial_binding_cleanup(
    function: &mut FunctionBuilder<'_>,
    instruction: &LoweredCleanupInstruction,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<(), BackendEmitError> {
    let LoweredCleanupInstruction::DropBindingFields { name, moved_fields, .. } = instruction
    else {
        return Ok(());
    };
    crate::codegen::cranelift::structs::emit_partial_binding_drop(
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
) -> Result<(), BackendEmitError> {
    let Some(NativeType::Arena(capacity)) =
        types.iter().find(|(binding, _)| binding.as_str() == name).map(|(_, ty)| *ty)
    else {
        return Err(BackendEmitError(format!("arena binding `{name}` is unavailable")));
    };
    let address = locals
        .iter()
        .find(|(binding, _)| binding.as_str() == name)
        .map(|(_, value)| *value)
        .ok_or_else(|| BackendEmitError(format!("arena binding `{name}` is unavailable")))?;
    let offset_address = function.ins().iadd_imm_s(address, i64::from(capacity));
    let zero = function.ins().iconst(layouts.pointer_type, 0);
    function.ins().store(MemFlagsData::new(), zero, offset_address, 0);
    Ok(())
}
