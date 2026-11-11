use crate::ast::Expr;
use crate::lexer::SourceSpan;

use super::super::{BackendContractError, BackendTargetContext, BackendUnsupportedOperation};

pub(super) fn expression_span(expression: &Expr) -> SourceSpan {
    match expression {
        Expr::Identifier { span, .. }
        | Expr::Integer { span, .. }
        | Expr::BoolLiteral { span, .. }
        | Expr::Grouping { span, .. }
        | Expr::Call { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Cast { span, .. }
        | Expr::Borrow { span, .. }
        | Expr::Try { span, .. }
        | Expr::StringLiteral { span, .. }
        | Expr::BufferLiteral { span, .. }
        | Expr::FloatLiteral { span, .. }
        | Expr::MethodCall { span, .. }
        | Expr::StructLit { span, .. }
        | Expr::FieldAccess { span, .. }
        | Expr::Index { span, .. }
        | Expr::Case { span, .. }
        | Expr::If { span, .. } => *span,
    }
}

pub(super) fn expression_kind(expression: &Expr) -> &'static str {
    match expression {
        Expr::Identifier { .. } => "identifier",
        Expr::Integer { .. } => "integer-literal",
        Expr::BoolLiteral { .. } => "bool-literal",
        Expr::Grouping { .. } => "grouping",
        Expr::Call { .. } => "call",
        Expr::Binary { .. } => "binary-expression",
        Expr::Unary { .. } => "unary-expression",
        Expr::Cast { .. } => "cast",
        Expr::Borrow { .. } => "borrow",
        Expr::Try { .. } => "try-expression",
        Expr::StringLiteral { .. } => "string-literal",
        Expr::BufferLiteral { .. } => "buffer-literal",
        Expr::FloatLiteral { .. } => "float-literal",
        Expr::MethodCall { .. } => "method-call",
        Expr::StructLit { .. } => "struct-literal",
        Expr::FieldAccess { .. } => "field-access",
        Expr::Index { .. } => "index-expression",
        Expr::Case { .. } => "case-expression",
        Expr::If { .. } => "if-expression",
    }
}

pub(super) fn unsupported(span: SourceSpan, reason: impl Into<String>) -> BackendContractError {
    unsupported_operation(span, "semantic-lowering", reason)
}

pub(super) fn unsupported_operation(
    span: SourceSpan,
    operation: impl Into<String>,
    reason: impl Into<String>,
) -> BackendContractError {
    BackendContractError::Unsupported(BackendUnsupportedOperation::new(
        operation,
        span,
        BackendTargetContext {
            backend: "neutral-contract".to_owned(),
            target: "semantic-model".to_owned(),
        },
        reason,
    ))
}
