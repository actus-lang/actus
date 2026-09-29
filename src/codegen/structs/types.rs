use std::collections::HashMap;

use crate::ast::{Expr, TypeName};

use super::super::layout::LayoutRegistry;
use super::super::types::NativeType;

pub(crate) fn expression_native_type(
    expression: &Expr,
    local_types: &HashMap<&String, NativeType>,
    layouts: &LayoutRegistry,
) -> Option<NativeType> {
    match expression {
        Expr::Identifier { name, .. } => local_types.get(name).copied(),
        Expr::StructLit { name, type_arguments, .. } => {
            let type_name = TypeName {
                name: name.clone(),
                arguments: type_arguments.clone(),
                reference_role: None,
                span: crate::lexer::SourceSpan::new(0, 0),
            };
            layouts.type_for_type_name(&type_name)
        }
        Expr::Grouping { expression, .. } | Expr::Borrow { expression, .. } => {
            expression_native_type(expression, local_types, layouts)
        }
        Expr::FieldAccess { object, field, .. } => {
            expression_native_type(object, local_types, layouts)
                .and_then(|ty| field_type(ty, field, layouts))
        }
        Expr::Index { target, .. } => expression_native_type(target, local_types, layouts)
            .and_then(|ty| match ty {
                NativeType::Array(id) => layouts.array(id).map(|array| array.element),
                NativeType::Buffer => Some(NativeType::Integer { signed: false, width: 8 }),
                _ => None,
            }),
        Expr::MethodCall { .. } => super::super::enums::enum_expression_type(expression, layouts),
        Expr::Integer { .. } => Some(NativeType::Int),
        Expr::FloatLiteral { .. } => Some(NativeType::Float { width: 64 }),
        Expr::StringLiteral { .. } => Some(NativeType::String),
        _ => None,
    }
}

pub(crate) fn field_type(
    ty: NativeType,
    field: &str,
    layouts: &LayoutRegistry,
) -> Option<NativeType> {
    match ty {
        NativeType::Struct(id) => layouts
            .get(id)?
            .fields
            .iter()
            .find(|candidate| candidate.name == field)
            .map(|candidate| candidate.ty),
        NativeType::Pack(id) => layouts
            .pack(id)?
            .fields
            .iter()
            .find(|candidate| candidate.name == field)
            .map(|candidate| candidate.ty),
        _ => None,
    }
}
