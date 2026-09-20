use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, types};
use cranelift_frontend::FunctionBuilder;

use crate::ast::{BinaryOp, Expr, Role, Stmt, UnaryOp};

use super::native::{FunctionRef, NativeEmitError};

#[derive(Clone, Copy)]
enum Flow {
    Fallthrough,
    Return(cranelift_codegen::ir::Value),
    Break,
    Continue,
}

#[derive(Clone, Copy)]
struct LoopTargets {
    header: cranelift_codegen::ir::Block,
    exit: cranelift_codegen::ir::Block,
}

pub(super) fn lower_body(
    function: &mut FunctionBuilder<'_>,
    statements: &[Stmt],
    initial_locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    functions: &HashMap<String, FunctionRef>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let mut locals = initial_locals.clone();
    match lower_statements(function, statements, &mut locals, functions, None)? {
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
) -> Result<Flow, NativeEmitError> {
    for statement in statements {
        let flow = lower_statement(function, statement, locals, functions, targets)?;
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
        Stmt::Return { value: Some(expression), .. } => {
            lower_expression(function, expression, locals, functions).map(Flow::Return)
        }
        Stmt::Return { value: None, .. } => Err(NativeEmitError(
            "native integer slice requires a return value".to_owned(),
        )),
        Stmt::Expression { expression, .. } => {
            lower_expression(function, expression, locals, functions)?;
            Ok(Flow::Fallthrough)
        }
        Stmt::Drop { .. } => Ok(Flow::Fallthrough),
        Stmt::Block(block) => {
            let mut nested_locals = locals.clone();
            lower_statements(function, &block.statements, &mut nested_locals, functions, targets)
        }
        Stmt::Loop(block) => lower_loop(function, block, locals, functions),
        Stmt::Break { .. } => emit_loop_jump(function, targets, false),
        Stmt::Continue { .. } => emit_loop_jump(function, targets, true),
        _ => Err(NativeEmitError(
            "native integer slice supports only integer declarations, assignments, expressions, blocks, drops, and returns"
                .to_owned(),
        )),
    }
}

fn emit_loop_jump(
    function: &mut FunctionBuilder<'_>,
    targets: Option<LoopTargets>,
    continue_loop: bool,
) -> Result<Flow, NativeEmitError> {
    let Some(targets) = targets else {
        let keyword = if continue_loop { "continue" } else { "break" };
        return Err(NativeEmitError(format!("{keyword} has no native loop target")));
    };
    let target = if continue_loop { targets.header } else { targets.exit };
    function.ins().jump(target, &[]);
    Ok(if continue_loop { Flow::Continue } else { Flow::Break })
}

fn lower_loop<'source>(
    function: &mut FunctionBuilder<'_>,
    block: &'source crate::ast::Block,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    functions: &HashMap<String, FunctionRef>,
) -> Result<Flow, NativeEmitError> {
    let header = function.create_block();
    let body = function.create_block();
    let exit = function.create_block();
    function.ins().jump(header, &[]);
    function.switch_to_block(header);
    function.ins().jump(body, &[]);
    function.seal_block(body);
    function.switch_to_block(body);
    let mut loop_locals = locals.clone();
    let flow = lower_statements(
        function,
        &block.statements,
        &mut loop_locals,
        functions,
        Some(LoopTargets { header, exit }),
    )?;
    if matches!(flow, Flow::Fallthrough | Flow::Continue) {
        function.ins().jump(header, &[]);
    }
    function.seal_block(header);
    if matches!(flow, Flow::Break | Flow::Fallthrough | Flow::Continue) {
        function.switch_to_block(exit);
        function.seal_block(exit);
        Ok(Flow::Fallthrough)
    } else {
        Ok(flow)
    }
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
