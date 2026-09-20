use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, types};
use cranelift_frontend::FunctionBuilder;

use crate::ast::{BinaryOp, Expr, Role, Stmt};

use super::native::{FunctionRef, NativeEmitError};

pub(super) fn lower_body(
    function: &mut FunctionBuilder<'_>,
    statements: &[Stmt],
    initial_locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    functions: &HashMap<String, FunctionRef>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let mut locals = initial_locals.clone();
    for statement in statements {
        match statement {
            Stmt::OwnerDecl { role: Role::Erg | Role::Abs, name, initializer, .. } => {
                let value = lower_expression(function, initializer, &locals, functions)?;
                locals.insert(name, value);
            }
            Stmt::Assignment { name, value, .. } => {
                let value = lower_expression(function, value, &locals, functions)?;
                locals.insert(name, value);
            }
            Stmt::Return { value: Some(expression), .. } => {
                return lower_expression(function, expression, &locals, functions);
            }
            Stmt::Return { value: None, .. } => {
                return Err(NativeEmitError(
                    "native integer slice requires a return value".to_owned(),
                ));
            }
            _ => {
                return Err(NativeEmitError(
                    "native integer slice supports only integer declarations, assignments, and returns"
                        .to_owned(),
                ));
            }
        }
    }
    Err(NativeEmitError("native integer slice requires a return value".to_owned()))
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
