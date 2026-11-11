use std::collections::HashMap;

use cranelift_codegen::ir::InstBuilder;
use cranelift_frontend::FunctionBuilder;

use crate::ast::{Expr, Stmt};
use crate::semantic::LoopExitKind;

use super::super::cleanup::{CleanupContext, emit_loop_cleanup, emit_return_cleanup};
use super::super::expressions::lower_expression;
use super::super::literals::StringDataValues;
use super::super::native::{BackendEmitError, FunctionRef};
use super::super::types::NativeType;
use super::{Flow, LoopTargets, LoweredCleanupSchedule};
use crate::codegen::cranelift::layout::LayoutRegistry;
use crate::codegen::cranelift::structs::emit_binding_drop;

mod compound;
mod control_flow;
mod dispatch;
mod owners;
mod structured_control;
mod try_lowering;

pub(crate) use compound::lower_compound_assignment;

#[allow(clippy::too_many_arguments)]
pub(crate) fn lower_statements<'source>(
    function: &mut FunctionBuilder<'_>,
    statements: &'source [Stmt],
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &mut HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    targets: Option<LoopTargets>,
    cleanup_schedule: &LoweredCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, BackendEmitError> {
    for statement in statements {
        let flow = dispatch::lower_statement(
            function,
            statement,
            locals,
            types,
            functions,
            targets.clone(),
            cleanup_schedule,
            string_data,
            layouts,
        )?;
        if !matches!(flow, Flow::Fallthrough) {
            return Ok(flow);
        }
    }
    Ok(Flow::Fallthrough)
}

#[allow(clippy::too_many_arguments)]
fn lower_return_statement(
    function: &mut FunctionBuilder<'_>,
    expression: Option<&Expr>,
    span: crate::lexer::SourceSpan,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &LoweredCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, BackendEmitError> {
    match expression {
        Some(expression) => try_lowering::lower_return(
            function,
            expression,
            span,
            locals,
            types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
        None => {
            lower_void_return(function, span, locals, types, functions, cleanup_schedule, layouts)
        }
    }
}

fn lower_void_return(
    function: &mut FunctionBuilder<'_>,
    span: crate::lexer::SourceSpan,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &LoweredCleanupSchedule,
    layouts: &LayoutRegistry,
) -> Result<Flow, BackendEmitError> {
    let mut cleanup = CleanupContext::new(function, locals, types, functions, layouts);
    emit_return_cleanup(&mut cleanup, cleanup_schedule, span, None)?;
    Ok(Flow::VoidReturn)
}

#[allow(clippy::too_many_arguments)]
fn lower_expression_statement(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    span: crate::lexer::SourceSpan,
    locals: &mut HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    targets: Option<LoopTargets>,
    cleanup_schedule: &LoweredCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, BackendEmitError> {
    if matches!(expression, Expr::Try { .. }) {
        return lower_try_expression_statement(
            function,
            expression,
            span,
            &*locals,
            types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        );
    }
    lower_plain_expression_statement(
        function,
        expression,
        locals,
        types,
        functions,
        targets,
        cleanup_schedule,
        string_data,
        layouts,
    )
}

#[allow(clippy::too_many_arguments)]
fn lower_plain_expression_statement(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    locals: &mut HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    targets: Option<LoopTargets>,
    cleanup_schedule: &LoweredCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, BackendEmitError> {
    if let Expr::Case { subject, branches, .. } = expression {
        super::super::case::lower_case(
            function,
            subject,
            branches,
            locals,
            types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
            targets,
            false,
        )?;
        return Ok(Flow::Fallthrough);
    }
    super::super::expressions::lower_expression_with_targets(
        function,
        expression,
        locals,
        types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
        targets,
    )?;
    Ok(Flow::Fallthrough)
}

#[allow(clippy::too_many_arguments)]
fn lower_try_expression_statement(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    span: crate::lexer::SourceSpan,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &LoweredCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, BackendEmitError> {
    try_lowering::lower_try_statement(
        function,
        expression,
        span,
        locals,
        types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )
}

#[allow(clippy::too_many_arguments)]
fn lower_loop_control(
    function: &mut FunctionBuilder<'_>,
    span: crate::lexer::SourceSpan,
    targets: Option<LoopTargets>,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &LoweredCleanupSchedule,
    layouts: &LayoutRegistry,
    continue_loop: bool,
) -> Result<Flow, BackendEmitError> {
    let exit_kind = if continue_loop { LoopExitKind::Continue } else { LoopExitKind::Break };
    let mut cleanup = CleanupContext::new(function, locals, types, functions, layouts);
    emit_loop_cleanup(&mut cleanup, cleanup_schedule, span, exit_kind)?;
    super::loops::emit_loop_jump(function, targets, locals, continue_loop)
}

#[allow(clippy::too_many_arguments)]
fn lower_assignment<'source>(
    function: &mut FunctionBuilder<'_>,
    name: &'source String,
    expression: &Expr,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &LoweredCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, BackendEmitError> {
    let value = lower_expression(
        function,
        expression,
        locals,
        types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    let value = types
        .get(name)
        .and_then(|ty| ty.ir_type(layouts.pointer_type).ok())
        .map_or(value, |target| {
            crate::codegen::cranelift::expressions::coerce_to_ir_type(function, value, target)
        });
    if locals.contains_key(name) {
        crate::codegen::cranelift::structs::emit_binding_drop(
            function, name, locals, types, functions, layouts,
        )?;
    }
    if store_assignment_value(function, name, value, locals, types, layouts)? {
        return Ok(Flow::Fallthrough);
    }
    locals.insert(name, value);
    Ok(Flow::Fallthrough)
}

fn store_assignment_value(
    function: &mut FunctionBuilder<'_>,
    name: &String,
    value: cranelift_codegen::ir::Value,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    layouts: &LayoutRegistry,
) -> Result<bool, BackendEmitError> {
    if let Some(destination) = locals.get(name)
        && types
            .get(name)
            .and_then(|ty| ty.ir_type(layouts.pointer_type).ok())
            .is_some_and(|ty| ty != layouts.pointer_type)
        && function.func.dfg.value_type(*destination) == layouts.pointer_type
    {
        function.ins().store(cranelift_codegen::ir::MemFlagsData::new(), value, *destination, 0);
        return Ok(true);
    }
    let Some(NativeType::Struct(id)) = types.get(name).copied() else { return Ok(false) };
    let Some(destination) = locals.get(name).copied() else { return Ok(false) };
    if !layouts.is_trivially_copyable_struct(id) {
        return Ok(false);
    }
    let size = layouts
        .type_size(NativeType::Struct(id))
        .ok_or_else(|| BackendEmitError("aggregate binding has no native size".to_owned()))?;
    crate::codegen::cranelift::structs::copy_bytes(function, value, destination, size, layouts)?;
    Ok(true)
}

fn lower_drop(
    function: &mut FunctionBuilder<'_>,
    name: &str,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<Flow, BackendEmitError> {
    emit_binding_drop(function, name, locals, types, functions, layouts)?;
    Ok(Flow::Fallthrough)
}
