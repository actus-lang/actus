use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, types};
use cranelift_frontend::FunctionBuilder;

use crate::ast::{BinaryOp, Expr, IntrinsicKind, UnaryOp, lookup_intrinsic};

use super::native::{FunctionRef, NativeEmitError};
use super::types::NativeType;

pub(super) fn lower_expression(
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
        Expr::Grouping { expression, .. } | Expr::Borrow { expression, .. } => {
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
        Expr::Call { callee, arguments, .. } => {
            lower_call(function, callee, arguments, locals, functions)
        }
        _ => Err(NativeEmitError(
            "native backend supports only integer and buffer expressions".to_owned(),
        )),
    }
}

fn lower_call(
    function: &mut FunctionBuilder<'_>,
    callee: &str,
    arguments: &[crate::ast::Argument],
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    functions: &HashMap<String, FunctionRef>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let target = functions
        .get(callee)
        .ok_or_else(|| NativeEmitError(format!("native function `{callee}` is unavailable")))?;
    let ordered = order_arguments(arguments, &target.parameter_names)?;
    let mut values = ordered
        .iter()
        .map(|argument| lower_expression(function, argument, locals, functions))
        .collect::<Result<Vec<_>, _>>()?;
    match lookup_intrinsic(callee) {
        Some(IntrinsicKind::Allocate) => {
            let length = values
                .pop()
                .ok_or_else(|| NativeEmitError("allocate requires a length".to_owned()))?;
            values.push(to_pointer_length(function, length));
        }
        Some(IntrinsicKind::Append) => {
            let byte =
                values.pop().ok_or_else(|| NativeEmitError("append requires a byte".to_owned()))?;
            values.push(to_byte(function, byte));
        }
        _ => {}
    }
    let call = function.ins().call(target.reference, &values);
    function
        .inst_results(call)
        .first()
        .copied()
        .ok_or_else(|| NativeEmitError(format!("native function `{callee}` returned no value")))
}

pub(super) fn initializer_type(
    expression: &Expr,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
) -> NativeType {
    match expression {
        Expr::Identifier { name, .. } => types.get(name).copied().unwrap_or(NativeType::Int),
        Expr::Grouping { expression, .. } | Expr::Borrow { expression, .. } => {
            initializer_type(expression, types, functions)
        }
        Expr::Call { callee, .. } => {
            functions.get(callee).map(|function| function.return_type).unwrap_or(NativeType::Int)
        }
        _ => NativeType::Int,
    }
}

pub(super) fn emit_buffer_drop(
    function: &mut FunctionBuilder<'_>,
    name: &str,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    functions: &HashMap<String, FunctionRef>,
) -> Result<(), NativeEmitError> {
    let value = locals
        .iter()
        .find(|(binding, _)| binding.as_str() == name)
        .map(|(_, value)| *value)
        .ok_or_else(|| NativeEmitError(format!("native binding `{name}` is unavailable")))?;
    let target = functions.get("actus_buffer_drop").ok_or_else(|| {
        NativeEmitError("native runtime function `actus_buffer_drop` is unavailable".to_owned())
    })?;
    function.ins().call(target.reference, &[value]);
    Ok(())
}

fn to_pointer_length(
    function: &mut FunctionBuilder<'_>,
    value: cranelift_codegen::ir::Value,
) -> cranelift_codegen::ir::Value {
    if function.func.dfg.value_type(value) == types::I64 {
        value
    } else {
        function.ins().uextend(types::I64, value)
    }
}

fn to_byte(
    function: &mut FunctionBuilder<'_>,
    value: cranelift_codegen::ir::Value,
) -> cranelift_codegen::ir::Value {
    if function.func.dfg.value_type(value) == types::I8 {
        value
    } else {
        function.ins().ireduce(types::I8, value)
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
