use std::collections::HashMap;

use crate::ast::Expr;

use super::super::enums::enum_expression_type;
use super::super::layout::LayoutRegistry;
use super::super::native::{FunctionRef, NativeEmitError};
use super::super::performance;
use super::super::structs::{expression_native_type, field_type};
use super::super::types::NativeType;

pub(crate) fn initializer_type(
    expression: &Expr,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<NativeType, NativeEmitError> {
    match expression {
        Expr::Identifier { name, .. } => types.get(name).copied().ok_or_else(|| {
            NativeEmitError(format!("native type for binding `{name}` is unavailable"))
        }),
        Expr::Integer { suffix, .. } => {
            Ok(suffix.as_deref().and_then(NativeType::from_name).unwrap_or(NativeType::Int))
        }
        Expr::BufferLiteral { .. } => Ok(NativeType::Buffer),
        Expr::FloatLiteral { .. } => Ok(NativeType::Float { width: 64 }),
        Expr::Grouping { expression, .. } | Expr::Borrow { expression, .. } => {
            initializer_type(expression, types, functions, layouts)
        }
        Expr::Cast { target, .. } => NativeType::from_type_name_with_layout(Some(target), layouts),
        Expr::Unary { expression, .. } => initializer_type(expression, types, functions, layouts),
        Expr::Binary { left, .. } => initializer_type(left, types, functions, layouts),
        Expr::StringLiteral { .. } => Ok(NativeType::String),
        Expr::Index { .. } => expression_native_type(expression, types, layouts)
            .ok_or_else(|| NativeEmitError("indexed expression has no native type".to_owned())),
        _ => infer_complex_initializer_type(expression, types, functions, layouts),
    }
}

fn infer_complex_initializer_type(
    expression: &Expr,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<NativeType, NativeEmitError> {
    match expression {
        Expr::Try { expression, .. } => infer_try_type(expression, types, functions, layouts),
        Expr::Call { callee, .. } => infer_call_type(callee, functions, layouts),
        Expr::MethodCall { .. } => infer_method_type(expression, types, functions, layouts),
        Expr::StructLit { name, type_arguments, .. } => {
            infer_struct_type(name, type_arguments, layouts)
        }
        Expr::FieldAccess { object, field, .. } => {
            infer_field_type(expression, object, field, types, layouts)
        }
        Expr::Case { branches, .. } => {
            super::super::case::infer_case_type(branches, types, functions, layouts)
        }
        Expr::If { then_branch, else_branch, .. } => {
            infer_if_type(then_branch, else_branch.as_ref(), types, functions, layouts)
        }
        _ => Err(NativeEmitError("unsupported native type expression".to_owned())),
    }
}

fn infer_if_type(
    then_branch: &crate::ast::Block,
    else_branch: Option<&crate::ast::IfBranch>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<NativeType, NativeEmitError> {
    let then_type = block_tail_type(then_branch, types, functions, layouts)?;
    let else_type = match else_branch {
        Some(crate::ast::IfBranch::Block(block)) => {
            block_tail_type(block, types, functions, layouts)?
        }
        Some(crate::ast::IfBranch::ElseIf(expression)) => {
            initializer_type(expression, types, functions, layouts)?
        }
        None => NativeType::Void,
    };
    if then_type == else_type {
        return Ok(then_type);
    }
    if matches!(then_type, NativeType::Void) {
        return Ok(else_type);
    }
    if matches!(else_type, NativeType::Void) {
        return Ok(then_type);
    }
    Err(NativeEmitError("if branches have different native types".to_owned()))
}

fn block_tail_type(
    block: &crate::ast::Block,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<NativeType, NativeEmitError> {
    let Some(crate::ast::Stmt::Expression { expression, span }) = block.statements.last() else {
        return Ok(NativeType::Void);
    };
    if span.end != expression_end(expression) {
        return Ok(NativeType::Void);
    }
    initializer_type(expression, types, functions, layouts)
}

fn expression_end(expression: &Expr) -> usize {
    match expression {
        Expr::Identifier { span, .. }
        | Expr::Integer { span, .. }
        | Expr::BufferLiteral { span, .. }
        | Expr::FloatLiteral { span, .. }
        | Expr::StringLiteral { span, .. }
        | Expr::Grouping { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Cast { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Borrow { span, .. }
        | Expr::Try { span, .. }
        | Expr::Call { span, .. }
        | Expr::MethodCall { span, .. }
        | Expr::StructLit { span, .. }
        | Expr::FieldAccess { span, .. }
        | Expr::Index { span, .. }
        | Expr::Case { span, .. }
        | Expr::If { span, .. } => span.end,
    }
}

fn infer_try_type(
    expression: &Expr,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<NativeType, NativeEmitError> {
    let NativeType::Enum(enum_id) = initializer_type(expression, types, functions, layouts)? else {
        return Err(NativeEmitError("try operand is not a native enum value".to_owned()));
    };
    layouts
        .enum_variant(enum_id, "Ok")
        .and_then(|variant| variant.fields.first().map(|field| field.ty))
        .ok_or_else(|| NativeEmitError("try operand has no Result.Ok payload".to_owned()))
}

fn infer_call_type(
    callee: &str,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<NativeType, NativeEmitError> {
    if let Some(id) = layouts.array_id(callee) {
        return Ok(NativeType::Array(id));
    }
    functions
        .get(callee)
        .map(|function| function.return_type)
        .ok_or_else(|| NativeEmitError(format!("native function `{callee}` is unavailable")))
}

fn infer_method_type(
    expression: &Expr,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<NativeType, NativeEmitError> {
    let Expr::MethodCall { receiver, method, .. } = expression else {
        return Err(NativeEmitError("native method expression is unavailable".to_owned()));
    };
    if let Some(enum_type) = enum_expression_type(expression, layouts) {
        return Ok(enum_type);
    }
    let receiver_type = initializer_type(receiver, types, functions, layouts)?;
    if matches!(receiver_type, NativeType::Arena(_)) && method == "place" {
        return arena_place_type(expression, types, functions, layouts);
    }
    let dispatch_name = performance::dispatch_key(receiver_type, method);
    functions
        .get(&dispatch_name)
        .map(|function| function.return_type)
        .or_else(|| functions.get(method).map(|function| function.return_type))
        .ok_or_else(|| NativeEmitError(format!("native method `{method}` is unavailable")))
}

fn infer_struct_type(
    name: &str,
    type_arguments: &[crate::ast::TypeName],
    layouts: &LayoutRegistry,
) -> Result<NativeType, NativeEmitError> {
    let type_name = crate::ast::TypeName {
        name: name.to_owned(),
        arguments: type_arguments.to_vec(),
        reference_role: None,
        span: crate::lexer::SourceSpan::new(0, 0),
    };
    layouts
        .type_for_type_name(&type_name)
        .ok_or_else(|| NativeEmitError(format!("native layout for struct `{name}` is unavailable")))
}

fn infer_field_type(
    expression: &Expr,
    object: &Expr,
    field: &str,
    types: &HashMap<&String, NativeType>,
    layouts: &LayoutRegistry,
) -> Result<NativeType, NativeEmitError> {
    expression_native_type(object, types, layouts)
        .and_then(|ty| field_type(ty, field, layouts))
        .or_else(|| enum_expression_type(expression, layouts))
        .ok_or_else(|| NativeEmitError(format!("native field `{field}` is unavailable")))
}

fn arena_place_type(
    expression: &Expr,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<NativeType, NativeEmitError> {
    let Expr::MethodCall { arguments, .. } = expression else {
        return Err(NativeEmitError("arena placement requires a method call".to_owned()));
    };
    arguments
        .first()
        .map(|argument| initializer_type(&argument.expression, types, functions, layouts))
        .ok_or_else(|| NativeEmitError("place requires one value".to_owned()))?
}
