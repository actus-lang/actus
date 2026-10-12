use crate::ast::{Block, Expr, Param, ReturnType, Stmt};
use crate::semantic::{GenericInstance, TypeSubstitution};

use super::definitions::canonical_type_name;

pub(super) fn specialize_param(param: &Param, substitution: &TypeSubstitution) -> Param {
    Param { ty: substitution.apply(&param.ty), ..param.clone() }
}

pub(super) fn specialize_return_type(
    return_type: &ReturnType,
    substitution: &TypeSubstitution,
) -> ReturnType {
    ReturnType { ty: substitution.apply(&return_type.ty), ..return_type.clone() }
}

pub(super) fn specialize_block(block: &Block, substitution: &TypeSubstitution) -> Block {
    Block {
        statements: block
            .statements
            .iter()
            .map(|statement| specialize_statement(statement, substitution))
            .collect(),
        span: block.span,
    }
}

fn specialize_statement(statement: &Stmt, substitution: &TypeSubstitution) -> Stmt {
    match statement {
        Stmt::OwnerDecl { .. } => specialize_owner_declaration(statement, substitution),
        Stmt::Assignment { target, value, span } => Stmt::Assignment {
            target: specialize_place(target, substitution),
            value: specialize_expression(value, substitution),
            span: *span,
        },
        Stmt::CompoundAssignment { target, operator, value, span } => Stmt::CompoundAssignment {
            target: specialize_place(target, substitution),
            operator: *operator,
            value: specialize_expression(value, substitution),
            span: *span,
        },
        Stmt::Expression { expression, span } => Stmt::Expression {
            expression: specialize_expression(expression, substitution),
            span: *span,
        },
        Stmt::If { condition, then_branch, else_branch, span } => Stmt::If {
            condition: specialize_expression(condition, substitution),
            then_branch: specialize_block(then_branch, substitution),
            else_branch: else_branch
                .as_ref()
                .map(|branch| specialize_if_branch(branch, substitution)),
            span: *span,
        },
        Stmt::Return { value: Some(value), span } => {
            Stmt::Return { value: Some(specialize_expression(value, substitution)), span: *span }
        }
        Stmt::Loop(block) => Stmt::Loop(specialize_block(block, substitution)),
        Stmt::ForRange { binding, start, end, body, span } => Stmt::ForRange {
            binding: binding.clone(),
            start: specialize_expression(start, substitution),
            end: specialize_expression(end, substitution),
            body: specialize_block(body, substitution),
            span: *span,
        },
        Stmt::ForArray { binding, collection, body, span } => Stmt::ForArray {
            binding: binding.clone(),
            collection: specialize_expression(collection, substitution),
            body: specialize_block(body, substitution),
            span: *span,
        },
        Stmt::Block(block) => Stmt::Block(specialize_block(block, substitution)),
        _ => statement.clone(),
    }
}

fn specialize_place(
    target: &crate::ast::Place,
    substitution: &TypeSubstitution,
) -> crate::ast::Place {
    match target {
        crate::ast::Place::Binding { name, span } => {
            crate::ast::Place::Binding { name: name.clone(), span: *span }
        }
        crate::ast::Place::Field { object, field, span } => crate::ast::Place::Field {
            object: Box::new(specialize_place(object, substitution)),
            field: field.clone(),
            span: *span,
        },
        crate::ast::Place::Index { target, index, span } => crate::ast::Place::Index {
            target: Box::new(specialize_place(target, substitution)),
            index: specialize_expression(index, substitution),
            span: *span,
        },
    }
}

fn specialize_owner_declaration(statement: &Stmt, substitution: &TypeSubstitution) -> Stmt {
    let Stmt::OwnerDecl { role, name, ty, span, initializer } = statement else { unreachable!() };
    Stmt::OwnerDecl {
        role: role.clone(),
        name: name.clone(),
        ty: ty.as_ref().map(|type_name| specialize_type_annotation(type_name, substitution)),
        initializer: specialize_expression(initializer, substitution),
        span: *span,
    }
}

fn specialize_type_annotation(type_name: &str, substitution: &TypeSubstitution) -> String {
    let Some((name, arguments)) = type_name.split_once('[') else {
        return substitution
            .apply_const(type_name)
            .map_or_else(|| type_name.to_owned(), str::to_owned);
    };
    let arguments = arguments.strip_suffix(']').unwrap_or(arguments);
    format!(
        "{name}[{}]",
        split_type_arguments(arguments)
            .into_iter()
            .map(|argument| specialize_type_argument(argument.trim(), substitution))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn specialize_if_branch(
    branch: &crate::ast::IfBranch,
    substitution: &TypeSubstitution,
) -> crate::ast::IfBranch {
    match branch {
        crate::ast::IfBranch::Block(block) => {
            crate::ast::IfBranch::Block(specialize_block(block, substitution))
        }
        crate::ast::IfBranch::ElseIf(expression) => {
            crate::ast::IfBranch::ElseIf(Box::new(specialize_expression(expression, substitution)))
        }
    }
}

fn specialize_expression(expression: &Expr, substitution: &TypeSubstitution) -> Expr {
    match expression {
        Expr::Identifier { name, span } => substitution.apply_const(name).map_or_else(
            || expression.clone(),
            |value| Expr::Integer { value: value.to_owned(), suffix: None, span: *span },
        ),
        Expr::Grouping { expression: child, span } => Expr::Grouping {
            expression: Box::new(specialize_expression(child, substitution)),
            span: *span,
        },
        Expr::Borrow { expression: child, span } => Expr::Borrow {
            expression: Box::new(specialize_expression(child, substitution)),
            span: *span,
        },
        Expr::Try { expression: child, span } => Expr::Try {
            expression: Box::new(specialize_expression(child, substitution)),
            span: *span,
        },
        Expr::Unary { operator, expression: child, span } => Expr::Unary {
            operator: *operator,
            expression: Box::new(specialize_expression(child, substitution)),
            span: *span,
        },
        Expr::Cast { expression: child, target, span } => Expr::Cast {
            expression: Box::new(specialize_expression(child, substitution)),
            target: substitution.apply(target),
            span: *span,
        },
        Expr::Binary { .. }
        | Expr::Call { .. }
        | Expr::MethodCall { .. }
        | Expr::StructLit { .. }
        | Expr::FieldAccess { .. }
        | Expr::Index { .. }
        | Expr::If { .. }
        | Expr::Case { .. } => specialize_complex_expression(expression, substitution),
        _ => expression.clone(),
    }
}

fn specialize_complex_expression(expression: &Expr, substitution: &TypeSubstitution) -> Expr {
    match expression {
        Expr::Binary { .. } => specialize_binary(expression, substitution),
        Expr::Call { .. } => specialize_call(expression, substitution),
        Expr::MethodCall { .. } => specialize_method_call(expression, substitution),
        Expr::StructLit { .. } => specialize_struct_literal(expression, substitution),
        Expr::FieldAccess { .. } => specialize_field_access(expression, substitution),
        Expr::Index { target, index, span } => Expr::Index {
            target: Box::new(specialize_expression(target, substitution)),
            index: Box::new(specialize_expression(index, substitution)),
            span: *span,
        },
        Expr::If { condition, then_branch, else_branch, span } => Expr::If {
            condition: Box::new(specialize_expression(condition, substitution)),
            then_branch: specialize_block(then_branch, substitution),
            else_branch: else_branch
                .as_ref()
                .map(|branch| specialize_if_branch(branch, substitution)),
            span: *span,
        },
        Expr::Case { .. } => specialize_case(expression, substitution),
        _ => expression.clone(),
    }
}

fn specialize_binary(expression: &Expr, substitution: &TypeSubstitution) -> Expr {
    let Expr::Binary { left, operator, right, span } = expression else { unreachable!() };
    Expr::Binary {
        left: Box::new(specialize_expression(left, substitution)),
        operator: *operator,
        right: Box::new(specialize_expression(right, substitution)),
        span: *span,
    }
}

fn specialize_call(expression: &Expr, substitution: &TypeSubstitution) -> Expr {
    let Expr::Call { callee, arguments, span } = expression else { unreachable!() };
    if callee.starts_with("size_of[") || callee.starts_with("align_of[") {
        let specialized = specialize_size_of_callee(callee, substitution);
        return Expr::Call { callee: specialized, arguments: Vec::new(), span: *span };
    }
    if (callee == "copy" || callee.starts_with("copy__"))
        && arguments.len() == 1
        && arguments[0].name.as_deref() == Some("value")
    {
        return Expr::Call {
            callee: "copy".to_owned(),
            arguments: specialize_arguments(arguments, substitution),
            span: *span,
        };
    }
    let callee = if callee.contains('[') {
        specialize_callee_name(callee, substitution)
    } else {
        substitution
            .apply_call_at(callee, *span)
            .or_else(|| substitution.apply_call(callee))
            .map_or_else(|| callee.clone(), str::to_owned)
    };
    Expr::Call { callee, arguments: specialize_arguments(arguments, substitution), span: *span }
}

fn specialize_size_of_callee(callee: &str, substitution: &TypeSubstitution) -> String {
    let Some((name, arguments)) = callee.split_once('[') else { return callee.to_owned() };
    let arguments = arguments.strip_suffix(']').unwrap_or(arguments);
    let arguments = split_type_arguments(arguments)
        .into_iter()
        .map(|argument| specialize_type_argument(argument.trim(), substitution))
        .collect::<Vec<_>>();
    format!("{name}[{}]", arguments.join(","))
}

fn specialize_callee_name(callee: &str, substitution: &TypeSubstitution) -> String {
    let Some((name, arguments)) = callee.split_once('[') else { return callee.to_owned() };
    let arguments = arguments.strip_suffix(']').unwrap_or(arguments);
    let arguments = split_type_arguments(arguments)
        .into_iter()
        .map(|argument| specialize_type_argument(argument.trim(), substitution))
        .collect::<Vec<_>>();
    if !name.chars().next().is_some_and(|character| character.is_ascii_lowercase()) {
        return format!("{name}[{}]", arguments.join(","));
    }
    let encoded = arguments
        .join("_")
        .chars()
        .map(|character| if character.is_ascii_alphanumeric() { character } else { '_' })
        .collect::<String>();
    format!("{name}__{encoded}")
}

fn specialize_type_argument(argument: &str, substitution: &TypeSubstitution) -> String {
    crate::semantic::parse_type_name_key(argument, crate::lexer::SourceSpan::new(0, 0))
        .map(|type_name| canonical_type_name(&substitution.apply(&type_name)))
        .unwrap_or_else(|| argument.to_owned())
}

fn split_type_arguments(arguments: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut depth = 0;
    for (index, character) in arguments.char_indices() {
        match character {
            '[' => depth += 1,
            ']' => depth -= 1,
            ',' if depth == 0 => {
                result.push(&arguments[start..index]);
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    result.push(&arguments[start..]);
    result
}

fn specialize_method_call(expression: &Expr, substitution: &TypeSubstitution) -> Expr {
    let Expr::MethodCall { receiver, method, arguments, span } = expression else { unreachable!() };
    Expr::MethodCall {
        receiver: Box::new(specialize_expression(receiver, substitution)),
        method: method.clone(),
        arguments: specialize_arguments(arguments, substitution),
        span: *span,
    }
}

fn specialize_struct_literal(expression: &Expr, substitution: &TypeSubstitution) -> Expr {
    let Expr::StructLit { name, type_arguments, fields, span } = expression else { unreachable!() };
    Expr::StructLit {
        name: name.clone(),
        type_arguments: type_arguments.iter().map(|ty| substitution.apply(ty)).collect(),
        fields: fields
            .iter()
            .map(|field| crate::ast::StructFieldInit {
                value: specialize_expression(&field.value, substitution),
                ..field.clone()
            })
            .collect(),
        span: *span,
    }
}

fn specialize_field_access(expression: &Expr, substitution: &TypeSubstitution) -> Expr {
    let Expr::FieldAccess { object, field, span } = expression else { unreachable!() };
    Expr::FieldAccess {
        object: Box::new(specialize_expression(object, substitution)),
        field: field.clone(),
        span: *span,
    }
}

fn specialize_case(expression: &Expr, substitution: &TypeSubstitution) -> Expr {
    let Expr::Case { mode, subject, branches, span } = expression else { unreachable!() };
    Expr::Case {
        mode: *mode,
        subject: Box::new(specialize_expression(subject, substitution)),
        branches: branches
            .iter()
            .map(|branch| crate::ast::CaseBranch {
                guard: branch
                    .guard
                    .as_ref()
                    .map(|guard| Box::new(specialize_expression(guard, substitution))),
                body: specialize_case_body(&branch.body, substitution),
                ..branch.clone()
            })
            .collect(),
        span: *span,
    }
}

fn specialize_case_body(
    body: &crate::ast::CaseBody,
    substitution: &TypeSubstitution,
) -> crate::ast::CaseBody {
    match body {
        crate::ast::CaseBody::Expression(value) => {
            crate::ast::CaseBody::Expression(Box::new(specialize_expression(value, substitution)))
        }
        crate::ast::CaseBody::Block(block) => {
            crate::ast::CaseBody::Block(specialize_block(block, substitution))
        }
    }
}

fn specialize_arguments(
    arguments: &[crate::ast::Argument],
    substitution: &TypeSubstitution,
) -> Vec<crate::ast::Argument> {
    arguments
        .iter()
        .map(|argument| crate::ast::Argument {
            expression: specialize_expression(&argument.expression, substitution),
            ..argument.clone()
        })
        .collect()
}

#[allow(dead_code)]
fn _specialization_key(instance: &GenericInstance) -> String {
    canonical_type_name(&crate::ast::TypeName {
        name: instance.name.clone(),
        arguments: instance.arguments.clone(),
        reference_role: None,
        span: crate::lexer::SourceSpan::new(0, 0),
    })
}
