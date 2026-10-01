use crate::ast::{Argument, Expr, PrimitiveType, lookup_builtin_type, primitive_type};
use crate::lexer::SourceSpan;

pub(super) fn is_scalar_name(name: &str) -> bool {
    matches!(
        lookup_builtin_type(name),
        Some(
            crate::ast::BuiltinType::Int
                | crate::ast::BuiltinType::Bool
                | crate::ast::BuiltinType::String
        )
    ) || matches!(
        primitive_type(name),
        Some(PrimitiveType::Integer { .. } | PrimitiveType::Float { .. })
    )
}

pub(super) fn argument_type_matches(expected: &str, found: &str, expression: &Expr) -> bool {
    let expected = strip_reference_role(expected);
    if strip_reference_role(found) == expected {
        return true;
    }
    match primitive_type(expected) {
        Some(PrimitiveType::Integer { .. }) => is_integer_literal(expression),
        Some(PrimitiveType::Float { .. }) => matches!(expression, Expr::FloatLiteral { .. }),
        Some(PrimitiveType::Void) => false,
        None => false,
    }
}

fn is_integer_literal(expression: &Expr) -> bool {
    match expression {
        Expr::Integer { .. } => true,
        Expr::Grouping { expression, .. } | Expr::Unary { expression, .. } => {
            is_integer_literal(expression)
        }
        _ => false,
    }
}

fn strip_reference_role(type_name: &str) -> &str {
    type_name
        .strip_prefix("abs ")
        .or_else(|| type_name.strip_prefix("ins "))
        .or_else(|| type_name.strip_prefix("erg "))
        .or_else(|| type_name.strip_prefix("dat "))
        .unwrap_or(type_name)
}

pub(super) fn argument_span(argument: &Argument) -> SourceSpan {
    expression_span(&argument.expression)
}

pub(super) fn expression_span(expression: &Expr) -> SourceSpan {
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
        | Expr::If { span, .. } => *span,
    }
}
