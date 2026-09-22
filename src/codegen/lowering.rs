use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, types};
use cranelift_frontend::FunctionBuilder;

use crate::ast::{Expr, Role, Stmt};
use crate::semantic::LoopExitKind;

use super::cleanup::{emit_loop_cleanup, emit_return_cleanup, emit_scope_cleanup};
use super::control_flow::{assigned_outer_bindings, carried_values, jump_with_values};
use super::expressions::{emit_buffer_drop, initializer_type, lower_expression};
use super::layout::LayoutRegistry;
use super::literals::StringDataValues;
use super::model::NativeCleanupSchedule;
use super::native::{FunctionRef, NativeEmitError};
use super::types::NativeType;

#[derive(Clone, Copy)]
enum Flow {
    Fallthrough,
    Return(cranelift_codegen::ir::Value),
    Break,
    Continue,
}

#[derive(Clone)]
struct LoopTargets {
    header: cranelift_codegen::ir::Block,
    exit: cranelift_codegen::ir::Block,
    carried: Vec<String>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_body(
    function: &mut FunctionBuilder<'_>,
    statements: &[Stmt],
    initial_locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    initial_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let mut locals = initial_locals.clone();
    let mut types = initial_types.clone();
    match lower_statements(
        function,
        statements,
        &mut locals,
        &mut types,
        functions,
        None,
        cleanup_schedule,
        string_data,
        layouts,
    )? {
        Flow::Return(value) => Ok(value),
        Flow::Fallthrough => {
            Err(NativeEmitError("native function requires a return value".to_owned()))
        }
        Flow::Break | Flow::Continue => {
            Err(NativeEmitError("loop control escaped its loop during native lowering".to_owned()))
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_statements<'source>(
    function: &mut FunctionBuilder<'_>,
    statements: &'source [Stmt],
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &mut HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    targets: Option<LoopTargets>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    for statement in statements {
        let flow = lower_statement(
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
fn lower_statement<'source>(
    function: &mut FunctionBuilder<'_>,
    statement: &'source Stmt,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &mut HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    targets: Option<LoopTargets>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    match statement {
        Stmt::OwnerDecl { role: Role::Erg | Role::Abs, name, ty, initializer, .. } =>
            lower_owner_declaration(function, name, ty.as_deref(), initializer, locals, types, functions, string_data, layouts),
        Stmt::Assignment { name, value, .. } => lower_assignment(function, name, value, locals, types, functions, string_data, layouts),
        Stmt::Return { value: Some(expression), span } =>
            lower_return(function, expression, *span, locals, types, functions, cleanup_schedule, string_data, layouts),
        Stmt::Return { value: None, .. } => {
            Err(NativeEmitError("native function requires a return value".to_owned()))
        }
        Stmt::Expression { expression, .. } => {
            lower_expression(function, expression, locals, types, functions, string_data, layouts)?;
            Ok(Flow::Fallthrough)
        }
        Stmt::Drop { name, .. } => lower_drop(function, name, locals, types, functions),
        Stmt::Block(block) => lower_scoped_block(
            function, block, locals, types, functions, targets, cleanup_schedule, string_data, layouts,
        ),
        Stmt::Loop(block) => lower_loop(function, block, locals, types, functions, cleanup_schedule, string_data, layouts),
        Stmt::Break { span } => {
            emit_loop_cleanup(function, cleanup_schedule, *span, LoopExitKind::Break, locals, types, functions)?;
            emit_loop_jump(function, targets, locals, false)
        }
        Stmt::Continue { span } => {
            emit_loop_cleanup(function, cleanup_schedule, *span, LoopExitKind::Continue, locals, types, functions)?;
            emit_loop_jump(function, targets, locals, true)
        }
        _ => Err(NativeEmitError(
            "native integer slice supports only integer declarations, assignments, expressions, blocks, drops, and returns"
                .to_owned(),
        )),
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_owner_declaration<'source>(
    function: &mut FunctionBuilder<'_>,
    name: &'source String,
    declared_type: Option<&str>,
    initializer: &Expr,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &mut HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    let value =
        lower_expression(function, initializer, locals, types, functions, string_data, layouts)?;
    locals.insert(name, value);
    let native_type = declared_type
        .and_then(NativeType::from_name)
        .unwrap_or_else(|| initializer_type(initializer, types, functions, layouts));
    types.insert(name, native_type);
    Ok(Flow::Fallthrough)
}

#[allow(clippy::too_many_arguments)]
fn lower_assignment<'source>(
    function: &mut FunctionBuilder<'_>,
    name: &'source String,
    expression: &Expr,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    let value =
        lower_expression(function, expression, locals, types, functions, string_data, layouts)?;
    locals.insert(name, value);
    Ok(Flow::Fallthrough)
}

#[allow(clippy::too_many_arguments)]
fn lower_return(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    span: crate::lexer::SourceSpan,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    let value =
        lower_expression(function, expression, locals, types, functions, string_data, layouts)?;
    emit_return_cleanup(function, cleanup_schedule, span, locals, types, functions)?;
    Ok(Flow::Return(value))
}

fn lower_drop<'source>(
    function: &mut FunctionBuilder<'_>,
    name: &'source String,
    locals: &HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
) -> Result<Flow, NativeEmitError> {
    if types.get(name) == Some(&NativeType::Buffer) {
        emit_buffer_drop(function, name, locals, functions)?;
    }
    Ok(Flow::Fallthrough)
}

#[allow(clippy::too_many_arguments)]
fn lower_scoped_block<'source>(
    function: &mut FunctionBuilder<'_>,
    block: &'source crate::ast::Block,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &mut HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    targets: Option<LoopTargets>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    let mut nested_locals = locals.clone();
    let mut nested_types = types.clone();
    let flow = lower_statements(
        function,
        &block.statements,
        &mut nested_locals,
        &mut nested_types,
        functions,
        targets,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    if matches!(flow, Flow::Fallthrough) {
        emit_scope_cleanup(
            function,
            cleanup_schedule,
            block,
            &nested_locals,
            &nested_types,
            functions,
        )?;
    }
    Ok(flow)
}

fn emit_loop_jump(
    function: &mut FunctionBuilder<'_>,
    targets: Option<LoopTargets>,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    continue_loop: bool,
) -> Result<Flow, NativeEmitError> {
    let Some(targets) = targets else {
        let keyword = if continue_loop { "continue" } else { "break" };
        return Err(NativeEmitError(format!("{keyword} has no native loop target")));
    };
    let target = if continue_loop { targets.header } else { targets.exit };
    let values = carried_values(locals, &targets.carried)?;
    jump_with_values(function, target, values);
    Ok(if continue_loop { Flow::Continue } else { Flow::Break })
}

#[allow(clippy::too_many_arguments)]
fn lower_loop<'source>(
    function: &mut FunctionBuilder<'_>,
    block: &'source crate::ast::Block,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &mut HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    let carried = assigned_outer_bindings(&block.statements, locals);
    let header = function.create_block();
    let body = function.create_block();
    let exit = function.create_block();
    for _ in &carried {
        function.append_block_param(header, types::I32);
        function.append_block_param(exit, types::I32);
    }
    let initial_values = carried_values(locals, &carried)?;
    jump_with_values(function, header, initial_values);
    function.switch_to_block(header);
    function.ins().jump(body, &[]);
    function.seal_block(body);
    function.switch_to_block(body);
    let mut loop_locals = locals.clone();
    let mut loop_types = types.clone();
    let header_values = function.block_params(header).to_vec();
    for (name, value) in carried.iter().zip(header_values) {
        let binding = locals.keys().find(|binding| binding.as_str() == name).unwrap();
        loop_locals.insert(binding, value);
    }
    let flow = lower_statements(
        function,
        &block.statements,
        &mut loop_locals,
        &mut loop_types,
        functions,
        Some(LoopTargets { header, exit, carried: carried.clone() }),
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    if matches!(flow, Flow::Fallthrough) {
        let values = carried_values(&loop_locals, &carried)?;
        jump_with_values(function, header, values);
    }
    function.seal_block(header);
    if matches!(flow, Flow::Break | Flow::Fallthrough | Flow::Continue) {
        function.switch_to_block(exit);
        function.seal_block(exit);
        let exit_values = function.block_params(exit).to_vec();
        for (name, value) in carried.iter().zip(exit_values) {
            let binding = locals.keys().find(|binding| binding.as_str() == name).unwrap();
            locals.insert(binding, value);
        }
        Ok(Flow::Fallthrough)
    } else {
        Ok(flow)
    }
}
