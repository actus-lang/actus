use crate::ast::{Expr, Stmt};
use std::collections::BTreeSet;

use super::CandidateContext;

pub(super) fn array_bindings(verb: &crate::ast::VerbDecl) -> BTreeSet<String> {
    let mut bindings: BTreeSet<String> = verb
        .params
        .iter()
        .filter(|parameter| fixed_array_type(&parameter.ty))
        .map(|parameter| parameter.name.clone())
        .collect();
    for statement in &verb.body.statements {
        if let Stmt::OwnerDecl { name, ty: Some(ty), .. } = statement
            && crate::semantic::parse_type_name_key(ty, crate::lexer::SourceSpan::new(0, 0))
                .is_some_and(|type_name| fixed_array_type(&type_name))
        {
            bindings.insert(name.clone());
        }
    }
    bindings
}

pub(super) fn local_array_bindings(verb: &crate::ast::VerbDecl) -> BTreeSet<String> {
    verb.body
        .statements
        .iter()
        .filter_map(|statement| match statement {
            Stmt::OwnerDecl { name, ty: Some(ty), .. }
                if crate::semantic::parse_type_name_key(
                    ty,
                    crate::lexer::SourceSpan::new(0, 0),
                )
                .is_some_and(|type_name| fixed_array_type(&type_name)) =>
            {
                Some(name.clone())
            }
            _ => None,
        })
        .collect()
}

pub(super) fn fixed_array_expression(expression: &Expr, context: &CandidateContext<'_>) -> bool {
    match expression {
        Expr::Identifier { name, .. } => context.array_bindings.contains(name),
        Expr::Call { callee, arguments, .. } => {
            arguments.is_empty()
                && callee.starts_with("Array[")
                && crate::semantic::parse_type_name_key(callee, crate::lexer::SourceSpan::new(0, 0))
                    .is_some_and(|type_name| fixed_array_type(&type_name))
        }
        _ => false,
    }
}

pub(super) fn fixed_array_type(type_name: &crate::ast::TypeName) -> bool {
    type_name.name == "Array"
        && type_name.arguments.len() == 2
        && type_name.arguments[1].name.parse::<u32>().is_ok_and(|capacity| {
            capacity > 0
                && scalar_type_size(&type_name.arguments[0]).is_some_and(|size| {
                    u64::from(size) * u64::from(capacity) <= u64::from(u32::MAX)
                })
        })
}

pub(super) fn fixed_array_index_expression(expression: &Expr) -> bool {
    matches!(expression, Expr::Integer { .. } | Expr::Identifier { .. })
}

fn scalar_type_size(type_name: &crate::ast::TypeName) -> Option<u32> {
    match type_name.name.as_str() {
        "Bool" | "u8" | "i8" => Some(1),
        "u16" | "i16" => Some(2),
        "Int" | "u32" | "i32" => Some(4),
        "Usize" | "u64" | "i64" => Some(8),
        _ => None,
    }
}
