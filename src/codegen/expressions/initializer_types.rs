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
        Expr::Integer { .. } => Ok(NativeType::Int),
        Expr::BufferLiteral { .. } => Ok(NativeType::Buffer),
        Expr::FloatLiteral { .. } => Ok(NativeType::Float { width: 64 }),
        Expr::Grouping { expression, .. } | Expr::Borrow { expression, .. } => {
            initializer_type(expression, types, functions, layouts)
        }
        Expr::Unary { expression, .. } => initializer_type(expression, types, functions, layouts),
        Expr::Binary { left, .. } => initializer_type(left, types, functions, layouts),
        Expr::StringLiteral { .. } => Ok(NativeType::String),
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
        Expr::Try { expression, .. } => {
            let NativeType::Enum(enum_id) =
                initializer_type(expression, types, functions, layouts)?
            else {
                return Err(NativeEmitError("try operand is not a native enum value".to_owned()));
            };
            layouts
                .enum_variant(enum_id, "Ok")
                .and_then(|variant| variant.fields.first().map(|field| field.ty))
                .ok_or_else(|| NativeEmitError("try operand has no Result.Ok payload".to_owned()))
        }
        Expr::Call { callee, .. } => functions
            .get(callee)
            .map(|function| function.return_type)
            .ok_or_else(|| NativeEmitError(format!("native function `{callee}` is unavailable"))),
        Expr::MethodCall { receiver, method, .. } => {
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
        Expr::StructLit { name, type_arguments, .. } => {
            let type_name = crate::ast::TypeName {
                name: name.clone(),
                arguments: type_arguments.clone(),
                reference_role: None,
                span: crate::lexer::SourceSpan::new(0, 0),
            };
            layouts.type_for_type_name(&type_name).ok_or_else(|| {
                NativeEmitError(format!("native layout for struct `{name}` is unavailable"))
            })
        }
        Expr::FieldAccess { object, field, .. } => expression_native_type(object, types, layouts)
            .and_then(|ty| field_type(ty, field, layouts))
            .or_else(|| enum_expression_type(expression, layouts))
            .ok_or_else(|| NativeEmitError(format!("native field `{field}` is unavailable"))),
        Expr::Case { branches, .. } => {
            super::super::case::infer_case_type(branches, types, functions, layouts)
        }
        _ => Err(NativeEmitError("unsupported native type expression".to_owned())),
    }
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
