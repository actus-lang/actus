use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, types};
use cranelift_frontend::FunctionBuilder;

use crate::ast::{BinaryOp, Expr, IntrinsicKind, UnaryOp, lookup_call_intrinsic};

use super::layout::LayoutRegistry;
use super::literals::StringDataValues;
use super::native::{FunctionRef, NativeEmitError};
use super::structs::{
    expression_native_type, field_type, lower_field_access, lower_struct_literal,
};
use super::types::NativeType;

pub(super) fn lower_expression(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    match expression {
        Expr::Integer { value, .. } => lower_integer(function, value),
        Expr::FloatLiteral { .. } => {
            Err(NativeEmitError("floating-point expressions are not lowered yet".to_owned()))
        }
        Expr::StringLiteral { value, .. } => lower_string(function, value, string_data),
        Expr::Identifier { name, .. } => lower_identifier(name, locals),
        Expr::Grouping { expression, .. } | Expr::Borrow { expression, .. } => lower_expression(
            function,
            expression,
            locals,
            local_types,
            functions,
            string_data,
            layouts,
        ),
        _ => lower_complex_expression(
            function,
            expression,
            locals,
            local_types,
            functions,
            string_data,
            layouts,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_complex_expression(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    match expression {
        Expr::Unary { .. } | Expr::Binary { .. } => lower_operation(
            function,
            expression,
            locals,
            local_types,
            functions,
            string_data,
            layouts,
        ),
        Expr::Call { .. } | Expr::StructLit { .. } | Expr::FieldAccess { .. } => lower_construct(
            function,
            expression,
            locals,
            local_types,
            functions,
            string_data,
            layouts,
        ),
        _ => Err(NativeEmitError("unsupported native expression".to_owned())),
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_operation(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    match expression {
        Expr::Unary { operator, expression, .. } => lower_unary(
            function,
            operator,
            expression,
            locals,
            local_types,
            functions,
            string_data,
            layouts,
        ),
        Expr::Binary { left, operator, right, .. } => lower_binary(
            function,
            left,
            operator,
            right,
            locals,
            local_types,
            functions,
            string_data,
            layouts,
        ),
        _ => Err(NativeEmitError("unsupported native operation".to_owned())),
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_construct(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    match expression {
        Expr::Call { callee, arguments, .. } => lower_call(
            function,
            callee,
            arguments,
            locals,
            local_types,
            functions,
            string_data,
            layouts,
        ),
        Expr::StructLit { name, fields, .. } => lower_struct_literal(
            function,
            name,
            fields,
            locals,
            local_types,
            functions,
            string_data,
            layouts,
        ),
        Expr::FieldAccess { object, field, .. } => lower_field_access(
            function,
            object,
            field,
            locals,
            local_types,
            functions,
            string_data,
            layouts,
        ),
        _ => Err(NativeEmitError("unsupported native construct".to_owned())),
    }
}

fn lower_integer(
    function: &mut FunctionBuilder<'_>,
    value: &str,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    value
        .parse::<i32>()
        .map(|value| function.ins().iconst(types::I32, i64::from(value)))
        .map_err(|error| NativeEmitError(format!("invalid integer literal: {error}")))
}

fn lower_string(
    function: &mut FunctionBuilder<'_>,
    value: &str,
    string_data: &StringDataValues,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    string_data
        .get(value)
        .copied()
        .map(|global| function.ins().symbol_value(types::I64, global))
        .ok_or_else(|| NativeEmitError(format!("string literal `{value}` has no native data")))
}

fn lower_identifier(
    name: &str,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    locals
        .iter()
        .find(|(binding, _)| binding.as_str() == name)
        .map(|(_, value)| *value)
        .ok_or_else(|| NativeEmitError(format!("native binding `{name}` is unavailable")))
}

#[allow(clippy::too_many_arguments)]
fn lower_unary(
    function: &mut FunctionBuilder<'_>,
    operator: &UnaryOp,
    expression: &Expr,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let value = lower_expression(
        function,
        expression,
        locals,
        local_types,
        functions,
        string_data,
        layouts,
    )?;
    match operator {
        UnaryOp::Negate => Ok(function.ins().ineg(value)),
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_binary(
    function: &mut FunctionBuilder<'_>,
    left: &Expr,
    operator: &BinaryOp,
    right: &Expr,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let left =
        lower_expression(function, left, locals, local_types, functions, string_data, layouts)?;
    let right =
        lower_expression(function, right, locals, local_types, functions, string_data, layouts)?;
    Ok(match operator {
        BinaryOp::Add => function.ins().iadd(left, right),
        BinaryOp::Subtract => function.ins().isub(left, right),
        BinaryOp::Multiply => function.ins().imul(left, right),
        BinaryOp::Divide => function.ins().sdiv(left, right),
    })
}

#[allow(clippy::too_many_arguments)]
fn lower_call(
    function: &mut FunctionBuilder<'_>,
    callee: &str,
    arguments: &[crate::ast::Argument],
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let target = functions
        .get(callee)
        .ok_or_else(|| NativeEmitError(format!("native function `{callee}` is unavailable")))?;
    let mut values = lower_call_arguments(
        function,
        arguments,
        &target.parameter_names,
        locals,
        local_types,
        functions,
        string_data,
        layouts,
    )?;
    let target = resolve_call_target(function, callee, target, &values, functions)?;
    normalize_call_arguments(function, callee, &mut values)?;
    let call = function.ins().call(target.reference, &values);
    function
        .inst_results(call)
        .first()
        .copied()
        .ok_or_else(|| NativeEmitError(format!("native function `{callee}` returned no value")))
}

#[allow(clippy::too_many_arguments)]
fn lower_call_arguments(
    function: &mut FunctionBuilder<'_>,
    arguments: &[crate::ast::Argument],
    parameter_names: &[String],
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Vec<cranelift_codegen::ir::Value>, NativeEmitError> {
    order_arguments(arguments, parameter_names)?
        .iter()
        .map(|argument| {
            lower_expression(
                function,
                argument,
                locals,
                local_types,
                functions,
                string_data,
                layouts,
            )
        })
        .collect()
}

fn resolve_call_target<'a>(
    function: &FunctionBuilder<'_>,
    callee: &str,
    target: &'a FunctionRef,
    values: &[cranelift_codegen::ir::Value],
    functions: &'a HashMap<String, FunctionRef>,
) -> Result<&'a FunctionRef, NativeEmitError> {
    if lookup_call_intrinsic(callee) == Some(IntrinsicKind::Print)
        && values.first().is_some_and(|value| function.func.dfg.value_type(*value) == types::I64)
    {
        functions.get("actus_print_string").ok_or_else(|| {
            NativeEmitError(
                "native runtime function `actus_print_string` is unavailable".to_owned(),
            )
        })
    } else {
        Ok(target)
    }
}

fn normalize_call_arguments(
    function: &mut FunctionBuilder<'_>,
    callee: &str,
    values: &mut Vec<cranelift_codegen::ir::Value>,
) -> Result<(), NativeEmitError> {
    match lookup_call_intrinsic(callee) {
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
    Ok(())
}

pub(super) fn initializer_type(
    expression: &Expr,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> NativeType {
    match expression {
        Expr::Identifier { name, .. } => types.get(name).copied().unwrap_or(NativeType::Int),
        Expr::Grouping { expression, .. } | Expr::Borrow { expression, .. } => {
            initializer_type(expression, types, functions, layouts)
        }
        Expr::Call { callee, .. } => {
            functions.get(callee).map(|function| function.return_type).unwrap_or(NativeType::Int)
        }
        Expr::StructLit { name, .. } => layouts.type_for_name(name).unwrap_or(NativeType::Int),
        Expr::FieldAccess { object, field, .. } => expression_native_type(object, types, layouts)
            .and_then(|ty| field_type(ty, field, layouts))
            .unwrap_or(NativeType::Int),
        Expr::FloatLiteral { .. } => NativeType::Int,
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
