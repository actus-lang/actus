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
            });
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
