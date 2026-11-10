use crate::ast::{Argument, EnumDef, EnumVariant, Expr};
use crate::lexer::SourceSpan;

use crate::semantic::errors::{SemanticError, SemanticErrorKind};

pub(super) fn strip_reference_role(type_name: &str) -> String {
    type_name
        .strip_prefix("abs ")
        .or_else(|| type_name.strip_prefix("ins "))
        .or_else(|| type_name.strip_prefix("erg "))
        .or_else(|| type_name.strip_prefix("dat "))
        .unwrap_or(type_name)
        .to_owned()
}

pub(super) fn named_argument_error(
    enum_name: &str,
    variant: &str,
    name: &str,
    argument: &Argument,
) -> SemanticError {
    SemanticError {
        kind: SemanticErrorKind::EnumVariantArgumentName {
            enum_name: enum_name.to_owned(),
            variant: variant.to_owned(),
            name: name.to_owned(),
        },
        span: argument_span(argument),
    }
}

pub(super) fn find_variant<'a>(definition: &'a EnumDef, name: &str) -> Option<&'a EnumVariant> {
    definition.variants.iter().find(|variant| variant.name == name)
}

pub(super) fn unknown_variant(enum_name: &str, variant: &str, span: SourceSpan) -> SemanticError {
    SemanticError {
        kind: SemanticErrorKind::UnknownEnumVariant {
            enum_name: enum_name.to_owned(),
            variant: variant.to_owned(),
        },
        span,
    }
}

pub(super) fn argument_count(
    enum_name: &str,
    variant: &str,
    expected: usize,
    found: usize,
    span: SourceSpan,
) -> SemanticError {
    SemanticError {
        kind: SemanticErrorKind::EnumVariantArgumentCount {
            enum_name: enum_name.to_owned(),
            variant: variant.to_owned(),
            expected,
            found,
        },
        span,
    }
}

pub(super) fn argument_span(argument: &Argument) -> SourceSpan {
    match &argument.expression {
        Expr::Identifier { span, .. }
        | Expr::Integer { span, .. }
        | Expr::BoolLiteral { span, .. }
        | Expr::BufferLiteral { span, .. }
        | Expr::FloatLiteral { span, .. }
        | Expr::StringLiteral { span, .. }
        | Expr::Grouping { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Borrow { span, .. }
        | Expr::Try { span, .. }
        | Expr::Call { span, .. }
        | Expr::MethodCall { span, .. }
        | Expr::StructLit { span, .. }
        | Expr::FieldAccess { span, .. }
        | Expr::Index { span, .. }
        | Expr::Case { span, .. }
        | Expr::If { span, .. }
        | Expr::Cast { span, .. } => *span,
    }
}
