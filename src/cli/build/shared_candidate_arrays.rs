use crate::ast::{Expr, Stmt};
use std::collections::BTreeSet;

use super::CandidateContext;

pub(super) fn array_bindings(
    verb: &crate::ast::VerbDecl,
    aggregate_types: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut bindings: BTreeSet<String> = verb
        .params
        .iter()
        .filter(|parameter| supported_array_type(&parameter.ty, aggregate_types))
        .map(|parameter| parameter.name.clone())
        .collect();
    for statement in &verb.body.statements {
        if let Stmt::OwnerDecl { name, ty: Some(ty), .. } = statement
            && crate::semantic::parse_type_name_key(ty, crate::lexer::SourceSpan::new(0, 0))
                .is_some_and(|type_name| supported_array_type(&type_name, aggregate_types))
        {
            bindings.insert(name.clone());
        }
    }
    bindings
}

pub(super) fn aggregate_array_bindings(
    verb: &crate::ast::VerbDecl,
    aggregate_types: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut bindings: BTreeSet<String> = verb
        .params
        .iter()
        .filter(|parameter| aggregate_array_type(&parameter.ty, aggregate_types))
        .map(|parameter| parameter.name.clone())
        .collect();
    for statement in &verb.body.statements {
        if let Stmt::OwnerDecl { name, ty: Some(ty), .. } = statement
            && crate::semantic::parse_type_name_key(ty, crate::lexer::SourceSpan::new(0, 0))
                .is_some_and(|type_name| aggregate_array_type(&type_name, aggregate_types))
        {
            bindings.insert(name.clone());
        }
    }
    bindings
}

pub(super) fn fixed_array_expression(expression: &Expr, context: &CandidateContext<'_>) -> bool {
    match expression {
        Expr::Identifier { name, .. } => context.array_bindings.contains(name),
        Expr::Call { callee, arguments, .. } => {
            arguments.is_empty()
                && callee.starts_with("Array[")
                && crate::semantic::parse_type_name_key(callee, crate::lexer::SourceSpan::new(0, 0))
                    .is_some_and(|type_name| {
                        supported_array_type(&type_name, context.aggregate_types)
                    })
        }
        _ => false,
    }
}

pub(super) fn supported_array_type(
    type_name: &crate::ast::TypeName,
    aggregate_types: &BTreeSet<String>,
) -> bool {
    type_name.name == "Array"
        && type_name.arguments.len() == 2
        && type_name.arguments[1].name.parse::<u32>().is_ok_and(|capacity| {
            capacity > 0
                && match scalar_type_size(&type_name.arguments[0]) {
                    Some(size) => u64::from(size) * u64::from(capacity) <= u64::from(u32::MAX),
                    None => aggregate_types.contains(&type_name.arguments[0].name),
                }
        })
}

fn aggregate_array_type(
    type_name: &crate::ast::TypeName,
    aggregate_types: &BTreeSet<String>,
) -> bool {
    type_name.name == "Array"
        && type_name.arguments.len() == 2
        && type_name.arguments[1].name.parse::<u32>().is_ok_and(|capacity| {
            capacity > 0 && aggregate_types.contains(&type_name.arguments[0].name)
        })
}

pub(super) fn fixed_array_index_expression(expression: &Expr) -> bool {
    matches!(expression, Expr::Integer { .. } | Expr::Identifier { .. })
}

pub(super) fn aggregate_array_expression(
    expression: &Expr,
    aggregate_array_bindings: &BTreeSet<String>,
    aggregate_types: &BTreeSet<String>,
) -> bool {
    let Expr::Index { target, index, .. } = expression else { return false };
    if !fixed_array_index_expression(index) {
        return false;
    }
    match target.as_ref() {
        Expr::Identifier { name, .. } => aggregate_array_bindings.contains(name),
        Expr::Call { callee, arguments, .. } if arguments.is_empty() => {
            crate::semantic::parse_type_name_key(callee, crate::lexer::SourceSpan::new(0, 0))
                .is_some_and(|type_name| {
                    type_name.name == "Array"
                        && type_name.arguments.len() == 2
                        && aggregate_types.contains(&type_name.arguments[0].name)
                })
        }
        _ => false,
    }
}

pub(super) fn integer_binding(binding: &crate::ast::ForBinding) -> bool {
    matches!(
        binding.ty.as_deref(),
        None | Some("u8") | Some("u16") | Some("u32") | Some("u64") | Some("Int")
    )
}

pub(super) fn scalar_place(place: &crate::ast::Place) -> bool {
    matches!(place, crate::ast::Place::Binding { .. } | crate::ast::Place::Field { .. })
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
