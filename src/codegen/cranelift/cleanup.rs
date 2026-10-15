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

pub(super) struct CleanupContext<'a, 'b> {
    pub(super) function: &'a mut FunctionBuilder<'b>,
    pub(super) locals: &'a HashMap<&'a String, cranelift_codegen::ir::Value>,
    pub(super) types: &'a HashMap<&'a String, NativeType>,
    pub(super) functions: &'a HashMap<String, FunctionRef>,
    pub(super) layouts: &'a LayoutRegistry,
}

impl<'a, 'b> CleanupContext<'a, 'b> {
    pub(super) fn new(
        function: &'a mut FunctionBuilder<'b>,
        locals: &'a HashMap<&'a String, cranelift_codegen::ir::Value>,
        types: &'a HashMap<&'a String, NativeType>,
        functions: &'a HashMap<String, FunctionRef>,
        layouts: &'a LayoutRegistry,
    ) -> Self {
        Self { function, locals, types, functions, layouts }
    }
}

pub(super) fn emit_return_cleanup(
    context: &mut CleanupContext<'_, '_>,
    schedule: &LoweredCleanupSchedule,
    span: SourceSpan,
    returned_expression: Option<&Expr>,
) -> Result<(), BackendEmitError> {
    let plan = schedule.return_plan(span).ok_or_else(|| {
        BackendEmitError(format!("missing native return cleanup plan at {span:?}"))
    })?;
    let returned_binding = returned_expression.and_then(returned_binding_name);
    for scope in &plan.scopes {
        emit_scope_instructions_filtered(
            context,
            scope,
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

pub(super) fn emit_loop_cleanup(
    context: &mut CleanupContext<'_, '_>,
    schedule: &LoweredCleanupSchedule,
    span: SourceSpan,
    kind: LoopExitKind,
) -> Result<(), BackendEmitError> {
    let plan = schedule
        .loop_plan(span, &kind)
        .ok_or_else(|| BackendEmitError(format!("missing native loop cleanup plan at {span:?}")))?;
    for scope in &plan.scopes {
        emit_scope_instructions(context, scope, schedule.resolved_plan(scope))?;
    }
    Ok(())
}

pub(super) fn emit_scope_cleanup(
    context: &mut CleanupContext<'_, '_>,
    schedule: &LoweredCleanupSchedule,
    span: SourceSpan,
) -> Result<(), BackendEmitError> {
    let plan = schedule.scope(span).ok_or_else(|| {
        BackendEmitError(format!("missing native scope cleanup plan at {:?}", span))
    })?;
    emit_scope_instructions(context, plan, schedule.resolved_plan(plan))?;
    Ok(())
}

fn emit_scope_instructions(
    context: &mut CleanupContext<'_, '_>,
    plan: &LoweredCleanupPlan,
    resolved: Option<&super::super::contract::BackendCleanupPlan>,
) -> Result<(), BackendEmitError> {
    for (index, instruction) in plan.instructions.iter().enumerate() {
        emit_cleanup_instruction(
            context,
            instruction,
            resolved.and_then(|plan| plan.resolved_bridges.get(index).map(Vec::as_slice)),
        )?;
    }
    Ok(())
}

fn emit_scope_instructions_filtered(
    context: &mut CleanupContext<'_, '_>,
    plan: &LoweredCleanupPlan,
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
            context,
            instruction,
            resolved.and_then(|plan| plan.resolved_bridges.get(index).map(Vec::as_slice)),
        )?;
    }
    Ok(())
}

fn emit_cleanup_instruction(
    context: &mut CleanupContext<'_, '_>,
    instruction: &LoweredCleanupInstruction,
    resolved: Option<&[BackendCleanupResolvedDescriptor]>,
) -> Result<(), BackendEmitError> {
    match instruction {
        LoweredCleanupInstruction::EndBorrow { .. } => Ok(()),
        LoweredCleanupInstruction::DropPayloadField { .. } => {
            emit_payload_cleanup(context, instruction)
        }
        LoweredCleanupInstruction::DropBinding { name, .. } => {
            if let Some(descriptor) = resolved.and_then(|descriptors| descriptors.first()) {
                emit_resolved_binding_drop(context, name, descriptor)?;
            } else {
                emit_binding_drop(
                    context.function,
                    name,
                    context.locals,
                    context.types,
                    context.functions,
                    context.layouts,
                )?;
            }
            Ok(())
        }
        LoweredCleanupInstruction::DropBindingFields { .. } => {
            emit_partial_binding_cleanup(context, instruction)
        }
        LoweredCleanupInstruction::ResetArena { name, .. } => {
            reset_arena(context.function, name, context.locals, context.types, context.layouts)
        }
    }
}

fn emit_resolved_binding_drop(
    context: &mut CleanupContext<'_, '_>,
    name: &str,
    descriptor: &BackendCleanupResolvedDescriptor,
) -> Result<(), BackendEmitError> {
    validate_resolved_binding_drop(descriptor)?;
    let value = context
        .locals
        .iter()
        .find(|(binding, _)| binding.as_str() == name)
        .map(|(_, value)| *value)
        .ok_or_else(|| BackendEmitError(format!("native binding `{name}` is unavailable")))?;
    let symbol = resolved_drop_symbol(descriptor)?;
    let target = context.functions.get(&symbol).ok_or_else(|| {
        BackendEmitError(format!("native runtime function `{symbol}` is unavailable"))
    })?;
    let mut arguments = vec![value];
    append_resolved_drop_arguments(context, descriptor, &mut arguments);
    context.function.ins().call(target.reference, &arguments);
    Ok(())
}

fn validate_resolved_binding_drop(
    descriptor: &BackendCleanupResolvedDescriptor,
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
    Ok(())
}

fn resolved_drop_symbol(
    descriptor: &BackendCleanupResolvedDescriptor,
) -> Result<String, BackendEmitError> {
    match descriptor.descriptor.bridge {
        super::super::contract::BackendCleanupBridge::BufferDrop => {
            Ok(crate::runtime::BUFFER_DROP_SYMBOL.to_owned())
        }
        super::super::contract::BackendCleanupBridge::RegionDrop => {
            Ok(super::performance::dispatch_key(NativeType::Region, "drop"))
        }
        super::super::contract::BackendCleanupBridge::EnumDrop { .. } => {
            Ok(crate::runtime::ENUM_DROP_SYMBOL.to_owned())
        }
        super::super::contract::BackendCleanupBridge::AggregateDrop { .. } => {
            Ok(crate::runtime::AGGREGATE_DROP_SYMBOL.to_owned())
        }
        super::super::contract::BackendCleanupBridge::PerformanceDrop => Err(BackendEmitError(
            "Cranelift user-defined Drop performance emission is not implemented".to_owned(),
        )),
    }
}

fn append_resolved_drop_arguments(
    context: &mut CleanupContext<'_, '_>,
    descriptor: &BackendCleanupResolvedDescriptor,
    arguments: &mut Vec<cranelift_codegen::ir::Value>,
) {
    if matches!(
        descriptor.descriptor.bridge,
        super::super::contract::BackendCleanupBridge::EnumDrop { .. }
    ) {
        arguments.push(
            context
                .function
                .ins()
                .iconst(context.layouts.pointer_type, i64::from(descriptor.layout.size)),
        );
    } else if matches!(
        descriptor.descriptor.bridge,
        super::super::contract::BackendCleanupBridge::AggregateDrop { .. }
    ) {
        arguments.push(
            context
                .function
                .ins()
                .iconst(context.layouts.pointer_type, i64::from(descriptor.layout.size)),
        );
        arguments.push(
            context
                .function
                .ins()
                .iconst(context.layouts.pointer_type, i64::from(descriptor.layout.alignment)),
        );
    }
}

fn emit_payload_cleanup(
    context: &mut CleanupContext<'_, '_>,
    instruction: &LoweredCleanupInstruction,
) -> Result<(), BackendEmitError> {
    let LoweredCleanupInstruction::DropPayloadField { name, enum_name, variant, field, .. } =
        instruction
    else {
        return Ok(());
    };
    super::enums::emit_enum_payload_drop(
        context.function,
        name,
        enum_name,
        variant,
        field,
        context.locals,
        context.types,
        context.functions,
        context.layouts,
    )
}

fn emit_partial_binding_cleanup(
    context: &mut CleanupContext<'_, '_>,
    instruction: &LoweredCleanupInstruction,
) -> Result<(), BackendEmitError> {
    let LoweredCleanupInstruction::DropBindingFields { name, moved_fields, .. } = instruction
    else {
        return Ok(());
    };
    crate::codegen::cranelift::structs::emit_partial_binding_drop(
        context.function,
        name,
        moved_fields,
        context.locals,
        context.types,
        context.functions,
        context.layouts,
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
