use crate::ast::{Expr, TypeName};
use crate::lexer::SourceSpan;

use super::super::{BackendContractError, BackendTargetContext, BackendUnsupportedOperation};

pub(super) fn parse_integer(value: &str) -> Option<u64> {
    let (negative, digits) =
        value.strip_prefix('-').map_or((false, value), |digits| (true, digits));
    let magnitude = digits
        .strip_prefix("0x")
        .map(|hex| u64::from_str_radix(hex, 16).ok())
        .unwrap_or_else(|| digits.parse().ok())?;
    Some(if negative { 0u64.wrapping_sub(magnitude) } else { magnitude })
}

pub(super) fn scalar_integer_type(type_name: &TypeName) -> Option<(bool, u8)> {
    match type_name.name.as_str() {
        "Int" => Some((true, 32)),
        "i8" => Some((true, 8)),
        "i16" => Some((true, 16)),
        "i32" => Some((true, 32)),
        "i64" => Some((true, 64)),
        "u8" => Some((false, 8)),
        "u16" => Some((false, 16)),
        "u32" => Some((false, 32)),
        "u64" => Some((false, 64)),
        _ => None,
    }
}

pub(super) fn scalar_backend_type(type_name: &TypeName) -> Option<super::super::BackendValueType> {
    if let Some(crate::ast::PrimitiveType::Float { width }) =
        crate::ast::primitive_type(&type_name.name)
    {
        return Some(super::super::BackendValueType::Float { width });
    }
    let (signed, width) = scalar_integer_type(type_name)?;
    Some(super::super::BackendValueType::Integer { signed, width })
}

pub(super) fn scalar_backend_type_name(name: &str) -> Option<super::super::BackendValueType> {
    let type_name = TypeName {
        name: name.to_owned(),
        arguments: Vec::new(),
        reference_role: None,
        span: crate::lexer::SourceSpan::new(0, 0),
    };
    scalar_backend_type(&type_name)
}

pub(super) fn backend_value_type(
    layouts: &(impl super::super::BackendLayoutProvider + ?Sized),
    type_name: &TypeName,
) -> Option<super::super::BackendValueType> {
    if type_name.reference_role.is_some() {
        return Some(super::super::BackendValueType::Pointer);
    }
    if type_name.name == "Bool" {
        return Some(super::super::BackendValueType::Bool);
    }
    if let Some(scalar) = scalar_backend_type(type_name) {
        return Some(scalar);
    }
    let layout = layouts.layout_for(type_name)?;
    if type_name.name == "Region" {
        return Some(super::super::BackendValueType::Region { layout });
    }
    if matches!(type_name.name.as_str(), "Arena" | "Buffer" | "String") {
        return Some(super::super::BackendValueType::Pointer);
    }
    if let Some(layout) = layouts.enum_layout_for(type_name) {
        if layout.niche_pointer {
            return Some(super::super::BackendValueType::NichePointerOption {
                layout: layout.layout,
                payload_layout: None,
            });
        }
        if layout.payload.is_none() {
            return Some(super::super::BackendValueType::Integer { signed: false, width: 32 });
        }
        return Some(super::super::BackendValueType::Enum { layout });
    }
    Some(super::super::BackendValueType::Aggregate { layout })
}

pub(super) fn integer_value_fits(value: u64, signed: bool, width: u8) -> bool {
    if signed {
        width == 64 || value <= ((1u64 << (width - 1)) - 1)
    } else {
        width == 64 || value <= ((1u64 << width) - 1)
    }
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
