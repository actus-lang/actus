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
use super::model::NativeCleanupSchedule;
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
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    match expression {
        Expr::Integer { value, .. } => lower_integer(function, value),
        Expr::BufferLiteral { length, .. } => lower_buffer_literal(
            function,
            length,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
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
            cleanup_schedule,
            string_data,
            layouts,
        ),
        _ => lower_complex_expression(
            function,
            expression,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_buffer_literal(
    function: &mut FunctionBuilder<'_>,
    length: &Expr,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let length = lower_expression(
        function,
        length,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    let length = if function.func.dfg.value_type(length) == layouts.pointer_type {
        length
    } else {
        function.ins().uextend(layouts.pointer_type, length)
    };
    let allocator = functions
        .get("__actus_buffer_allocate")
        .ok_or_else(|| NativeEmitError("native buffer allocator is unavailable".to_owned()))?;
    let call = function.ins().call(allocator.reference, &[length]);
    function
        .inst_results(call)
        .first()
        .copied()
        .ok_or_else(|| NativeEmitError("native buffer allocator returned no value".to_owned()))
}

#[allow(clippy::too_many_arguments)]
fn lower_complex_expression(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
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
            cleanup_schedule,
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
            cleanup_schedule,
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
            cleanup_schedule,
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
        Expr::BufferLiteral { .. } => NativeType::Buffer,
        Expr::Grouping { expression, .. } | Expr::Borrow { expression, .. } => {
            initializer_type(expression, types, functions, layouts)
        }
        Expr::Call { callee, .. } => {
            functions.get(callee).map(|function| function.return_type).unwrap_or(NativeType::Int)
        }
        Expr::MethodCall { method, .. } => enum_expression_type(expression, layouts)
            .or_else(|| functions.get(method).map(|function| function.return_type))
            .unwrap_or(NativeType::Int),
        Expr::StructLit { name, type_arguments, .. } => {
            let type_name = crate::ast::TypeName {
                name: name.clone(),
                arguments: type_arguments.clone(),
                span: crate::lexer::SourceSpan::new(0, 0),
            };
            layouts.type_for_type_name(&type_name).unwrap_or(NativeType::Int)
        }
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
