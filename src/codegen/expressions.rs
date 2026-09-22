use std::collections::HashMap;

use cranelift_codegen::ir::InstBuilder;
use cranelift_frontend::FunctionBuilder;

use crate::ast::Expr;

use super::case::lower_case;
use super::enums::enum_expression_type;
use super::expression_construct::lower_construct;
use super::expression_literals::{lower_identifier, lower_integer, lower_string};
use super::expression_operations::lower_operation;
use super::layout::LayoutRegistry;
use super::literals::StringDataValues;
use super::native::{FunctionRef, NativeEmitError};
use super::structs::{expression_native_type, field_type};
use super::types::NativeType;

#[allow(clippy::too_many_arguments)]
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
        Expr::Call { .. }
        | Expr::MethodCall { .. }
        | Expr::StructLit { .. }
        | Expr::FieldAccess { .. } => lower_construct(
            function,
            expression,
            locals,
            local_types,
            functions,
            string_data,
            layouts,
        ),
        Expr::Case { subject, branches, .. } => lower_case(
            function,
            subject,
            branches,
            locals,
            local_types,
            functions,
            string_data,
            layouts,
        ),
        _ => Err(NativeEmitError("unsupported native expression".to_owned())),
    }
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
        Expr::MethodCall { method, .. } => enum_expression_type(expression, layouts)
            .or_else(|| functions.get(method).map(|function| function.return_type))
            .unwrap_or(NativeType::Int),
        Expr::StructLit { name, .. } => layouts.type_for_name(name).unwrap_or(NativeType::Int),
        Expr::FieldAccess { object, field, .. } => expression_native_type(object, types, layouts)
            .and_then(|ty| field_type(ty, field, layouts))
            .or_else(|| enum_expression_type(expression, layouts))
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
