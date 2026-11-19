use crate::ast::{Argument, ArgumentRoleResolution, DeclarationIdentity, Expr, Role, TypeName};
use crate::lexer::SourceSpan;

pub(super) fn integer(value: u64, suffix: Option<&str>, span: SourceSpan) -> Expr {
    Expr::Integer { value: value.to_string(), suffix: suffix.map(str::to_owned), span }
}

pub(super) fn synthetic_span(span: SourceSpan, offset: usize) -> SourceSpan {
    SourceSpan::new(span.end.saturating_add(offset), span.end.saturating_add(offset + 1))
}

pub(super) fn argument(name: Option<&str>, role: Role, expression: Expr) -> Argument {
    Argument {
        name: name.map(str::to_owned),
        role: Some(role),
        role_span: Some(expression_span(&expression)),
        role_resolution: ArgumentRoleResolution::Explicit,
        expression,
    }
}

pub(super) fn inferred_argument(expression: Expr) -> Argument {
    Argument {
        name: None,
        role: None,
        role_span: None,
        role_resolution: ArgumentRoleResolution::Unspecified,
        expression,
    }
}

pub(super) fn identifier(name: &str, span: SourceSpan) -> Expr {
    Expr::Identifier { name: name.to_owned(), span }
}

pub(super) fn type_name(name: &str, span: SourceSpan) -> TypeName {
    assert!(is_builtin(name), "internal compiler error: generated type `{name}` is not builtin");
    TypeName::builtin(name, Vec::new(), None, span)
}

pub(super) fn type_name_with_identity(
    name: &str,
    identity: Option<&DeclarationIdentity>,
    span: SourceSpan,
) -> TypeName {
    match identity {
        Some(identity) => TypeName::resolved(identity.clone(), name, Vec::new(), None, span),
        None => type_name(name, span),
    }
}

pub(super) fn result_type(value: &str, span: SourceSpan) -> TypeName {
    TypeName::builtin(
        "Result",
        vec![type_name(value, span), type_name("SerializationError", span)],
        None,
        span,
    )
}

pub(super) fn result_type_with_value(value: &TypeName, span: SourceSpan) -> TypeName {
    TypeName::builtin(
        "Result",
        vec![value.clone(), type_name("SerializationError", span)],
        None,
        span,
    )
}

fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "Result"
            | "Option"
            | "Box"
            | "Array"
            | "Buffer"
            | "Int"
            | "Bool"
            | "String"
            | "Usize"
            | "SerializationError"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
    )
}

fn expression_span(expression: &Expr) -> SourceSpan {
    match expression {
        Expr::Identifier { span, .. }
        | Expr::Integer { span, .. }
        | Expr::FieldAccess { span, .. }
        | Expr::Index { span, .. }
        | Expr::Call { span, .. }
        | Expr::MethodCall { span, .. } => *span,
        _ => SourceSpan::new(0, 0),
    }
}
