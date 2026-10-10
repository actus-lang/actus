use crate::ast::{Expr, PrimitiveType, TypeName};
use crate::lexer::SourceSpan;

use super::super::{
    BackendContractError, BackendTargetContext, BackendUnsupportedOperation, BackendValueType,
};

pub(super) fn scalar_type(
    type_name: &TypeName,
) -> Result<Option<BackendValueType>, BackendContractError> {
    let ty = match type_name.name.as_str() {
        "Bool" => BackendValueType::Bool,
        "Int" => BackendValueType::Integer { signed: true, width: 32 },
        "Usize" => BackendValueType::Integer { signed: false, width: 64 },
        _ => match crate::ast::primitive_type(&type_name.name) {
            Some(PrimitiveType::Integer { signed, width }) => {
                BackendValueType::Integer { signed, width }
            }
            Some(PrimitiveType::Void) => return Ok(None),
            Some(PrimitiveType::Float { .. }) => {
                return Err(unsupported(
                    type_name.span,
                    "floating-point shared lowering is disabled",
                ));
            }
            None => return Err(unsupported(type_name.span, "non-scalar type in scalar lowering")),
        },
    };
    Ok(Some(ty))
}

pub(super) fn required_scalar_type(
    type_name: &TypeName,
) -> Result<BackendValueType, BackendContractError> {
    scalar_type(type_name)?
        .ok_or_else(|| unsupported(type_name.span, "unit type is not valid here"))
}

pub(super) fn parse_integer(value: &str) -> Option<u64> {
    value
        .strip_prefix("0x")
        .map(|hex| u64::from_str_radix(hex, 16).ok())
        .unwrap_or_else(|| value.parse().ok())
}

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

pub(super) fn unsupported(span: SourceSpan, reason: impl Into<String>) -> BackendContractError {
    BackendContractError::Unsupported(BackendUnsupportedOperation::new(
        "semantic-lowering",
        span,
        BackendTargetContext {
            backend: "neutral-contract".to_owned(),
            target: "semantic-model".to_owned(),
        },
        reason,
    ))
}
