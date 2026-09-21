use std::collections::HashMap;

use cranelift_codegen::ir::{BlockArg, InstBuilder, types};
use cranelift_frontend::FunctionBuilder;

use crate::ast::{BinaryOp, Expr, Role, Stmt, UnaryOp};
use crate::semantic::LoopExitKind;

use super::cleanup::{emit_loop_cleanup, emit_return_cleanup, emit_scope_cleanup};
use super::model::NativeCleanupSchedule;
use super::native::{FunctionRef, NativeEmitError};

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

pub(super) fn lower_body(
    function: &mut FunctionBuilder<'_>,
    statements: &[Stmt],
    initial_locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let mut locals = initial_locals.clone();
    match lower_statements(function, statements, &mut locals, functions, None, cleanup_schedule)? {
        Flow::Return(value) => Ok(value),
        Flow::Fallthrough => {
            Err(NativeEmitError("native integer slice requires a return value".to_owned()))
        }
        Flow::Break | Flow::Continue => {
            Err(NativeEmitError("loop control escaped its loop during native lowering".to_owned()))
        }
    }
}

fn lower_statements<'source>(
    function: &mut FunctionBuilder<'_>,
    statements: &'source [Stmt],
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    functions: &HashMap<String, FunctionRef>,
    targets: Option<LoopTargets>,
    cleanup_schedule: &NativeCleanupSchedule,
) -> Result<Flow, NativeEmitError> {
    for statement in statements {
        let flow = lower_statement(
            function,
            statement,
            locals,
            functions,
            targets.clone(),
            cleanup_schedule,
        )?;
        if !matches!(flow, Flow::Fallthrough) {
            return Ok(flow);
        }
    }
    Ok(Flow::Fallthrough)
}

fn lower_statement<'source>(
    function: &mut FunctionBuilder<'_>,
    statement: &'source Stmt,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    functions: &HashMap<String, FunctionRef>,
    targets: Option<LoopTargets>,
    cleanup_schedule: &NativeCleanupSchedule,
) -> Result<Flow, NativeEmitError> {
    match statement {
        Stmt::OwnerDecl { role: Role::Erg | Role::Abs, name, initializer, .. } => {
            let value = lower_expression(function, initializer, locals, functions)?;
            locals.insert(name, value);
            Ok(Flow::Fallthrough)
        }
        Stmt::Assignment { name, value, .. } => {
            let value = lower_expression(function, value, locals, functions)?;
            locals.insert(name, value);
            Ok(Flow::Fallthrough)
        }
        Stmt::Return { value: Some(expression), span } => {
            let value = lower_expression(function, expression, locals, functions)?;
            emit_return_cleanup(function, cleanup_schedule, *span)?;
            Ok(Flow::Return(value))
        }
        Stmt::Return { value: None, .. } => Err(NativeEmitError(
            "native integer slice requires a return value".to_owned(),
        )),
        Stmt::Expression { expression, .. } => {
            lower_expression(function, expression, locals, functions)?;
            Ok(Flow::Fallthrough)
        }
        Stmt::Drop { .. } => Ok(Flow::Fallthrough),
        Stmt::Block(block) => lower_scoped_block(
            function,
            block,
            locals,
            functions,
            targets,
            cleanup_schedule,
        ),
        Stmt::Loop(block) => lower_loop(function, block, locals, functions, cleanup_schedule),
        Stmt::Break { span } => {
            emit_loop_cleanup(function, cleanup_schedule, *span, LoopExitKind::Break)?;
            emit_loop_jump(function, targets, locals, false)
        }
        Stmt::Continue { span } => {
            emit_loop_cleanup(function, cleanup_schedule, *span, LoopExitKind::Continue)?;
            emit_loop_jump(function, targets, locals, true)
        }
        _ => Err(NativeEmitError(
            "native integer slice supports only integer declarations, assignments, expressions, blocks, drops, and returns"
                .to_owned(),
        )),
    }
}

fn lower_scoped_block<'source>(
    function: &mut FunctionBuilder<'_>,
    block: &'source crate::ast::Block,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    functions: &HashMap<String, FunctionRef>,
    targets: Option<LoopTargets>,
    cleanup_schedule: &NativeCleanupSchedule,
) -> Result<Flow, NativeEmitError> {
    let mut nested_locals = locals.clone();
    let flow = lower_statements(
        function,
        &block.statements,
        &mut nested_locals,
        functions,
        targets,
        cleanup_schedule,
    )?;
    if matches!(flow, Flow::Fallthrough) {
        emit_scope_cleanup(function, cleanup_schedule, block)?;
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

fn lower_loop<'source>(
    function: &mut FunctionBuilder<'_>,
    block: &'source crate::ast::Block,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
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
    let header_values = function.block_params(header).to_vec();
    for (name, value) in carried.iter().zip(header_values) {
        let binding = locals.keys().find(|binding| binding.as_str() == name).unwrap();
        loop_locals.insert(binding, value);
    }
    let flow = lower_statements(
        function,
        &block.statements,
        &mut loop_locals,
        functions,
        Some(LoopTargets { header, exit, carried: carried.clone() }),
        cleanup_schedule,
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

fn jump_with_values(
    function: &mut FunctionBuilder<'_>,
    target: cranelift_codegen::ir::Block,
    values: Vec<cranelift_codegen::ir::Value>,
) {
    let arguments = values.into_iter().map(BlockArg::Value).collect::<Vec<_>>();
    function.ins().jump(target, arguments.iter());
}

fn assigned_outer_bindings(
    statements: &[Stmt],
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
) -> Vec<String> {
    let mut names = Vec::new();
    for statement in statements {
        let name = match statement {
            Stmt::Assignment { name, .. }
                if locals.keys().any(|binding| binding.as_str() == name) =>
            {
                Some(name)
            }
            Stmt::Block(block) | Stmt::Loop(block) => {
                names.extend(assigned_outer_bindings(&block.statements, locals));
                None
            }
            _ => None,
        };
        if let Some(name) = name {
            names.push(name.clone());
        }
    }
    names.sort();
    names.dedup();
    names
}

fn carried_values(
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    names: &[String],
) -> Result<Vec<cranelift_codegen::ir::Value>, NativeEmitError> {
    names
        .iter()
        .map(|name| {
            locals
                .iter()
                .find(|(binding, _)| binding.as_str() == name)
                .map(|(_, value)| *value)
                .ok_or_else(|| NativeEmitError(format!("native binding `{name}` is unavailable")))
        })
        .collect()
}

fn lower_expression(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    functions: &HashMap<String, FunctionRef>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    match expression {
        Expr::Integer { value, .. } => value
            .parse::<i32>()
            .map(|value| function.ins().iconst(types::I32, i64::from(value)))
            .map_err(|error| NativeEmitError(format!("invalid integer literal: {error}"))),
        Expr::Identifier { name, .. } => locals
            .get(name)
            .copied()
            .ok_or_else(|| NativeEmitError(format!("native binding `{name}` is unavailable"))),
        Expr::Grouping { expression, .. } => {
            lower_expression(function, expression, locals, functions)
        }
        Expr::Unary { operator, expression, .. } => {
            let value = lower_expression(function, expression, locals, functions)?;
            match operator {
                UnaryOp::Negate => Ok(function.ins().ineg(value)),
            }
        }
        Expr::Binary { left, operator, right, .. } => {
            let left = lower_expression(function, left, locals, functions)?;
            let right = lower_expression(function, right, locals, functions)?;
            let value = match operator {
                BinaryOp::Add => function.ins().iadd(left, right),
                BinaryOp::Subtract => function.ins().isub(left, right),
                BinaryOp::Multiply => function.ins().imul(left, right),
                BinaryOp::Divide => function.ins().sdiv(left, right),
            };
            Ok(value)
        }
        Expr::Borrow { expression, .. } => {
            lower_expression(function, expression, locals, functions)
        }
        Expr::Call { callee, arguments, .. } => {
            let target = functions.get(callee).ok_or_else(|| {
                NativeEmitError(format!("native function `{callee}` is unavailable"))
            })?;
            let ordered = order_arguments(arguments, &target.parameter_names)?;
            let values = ordered
                .iter()
                .map(|argument| lower_expression(function, argument, locals, functions))
                .collect::<Result<Vec<_>, _>>()?;
            let call = function.ins().call(target.reference, &values);
            function.inst_results(call).first().copied().ok_or_else(|| {
                NativeEmitError(format!("native function `{callee}` returned no value"))
            })
        }
        _ => Err(NativeEmitError(
            "native integer slice supports only integer expressions".to_owned(),
        )),
    }
}

fn order_arguments<'source>(
    arguments: &'source [crate::ast::Argument],
    parameter_names: &[String],
) -> Result<Vec<&'source Expr>, NativeEmitError> {
    if arguments.iter().all(|argument| argument.name.is_none()) {
        return Ok(arguments.iter().map(|argument| &argument.expression).collect());
    }
    if arguments.iter().any(|argument| argument.name.is_none()) {
        return Err(NativeEmitError(
            "native calls cannot mix named and positional arguments".to_owned(),
        ));
    }
    parameter_names
        .iter()
        .map(|parameter| {
            arguments
                .iter()
                .find(|argument| argument.name.as_deref() == Some(parameter))
                .map(|argument| &argument.expression)
                .ok_or_else(|| NativeEmitError(format!("missing native argument `{parameter}")))
        })
        .collect()
}
