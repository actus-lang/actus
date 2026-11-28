use crate::ast::{
    BuiltinType, PrimitiveType, StructField, StructFieldRole, lookup_builtin_type, primitive_type,
};
use crate::lexer::SourceSpan;

use super::super::errors::{SemanticError, SemanticErrorKind};

/// Whether `name` is a scalar that has no view: `Int`, `Bool`, `Usize`, a
/// fixed-width integer or a float. `String`, `Buffer` and aggregates can be
/// viewed and are not scalars for this rule (ADR-0046 2.3, E1147).
fn is_scalar_view_type(name: &str) -> bool {
    matches!(lookup_builtin_type(name), Some(BuiltinType::Int | BuiltinType::Bool))
        || matches!(
            primitive_type(name),
            Some(PrimitiveType::Integer { .. } | PrimitiveType::Float { .. })
        )
}

/// Validates the ownership role of one struct field declaration. An `ins` field
/// is rejected with `E1094`, and an `abs` field of a scalar type with `E1147`.
pub(super) fn validate_field_role(
    struct_name: &str,
    field: &StructField,
) -> Result<(), SemanticError> {
    if field.role == StructFieldRole::Abs && is_scalar_view_type(&field.ty.name) {
        return Err(SemanticError {
            kind: SemanticErrorKind::ScalarViewField {
                struct_name: struct_name.to_owned(),
                field: field.name.clone(),
                type_name: field.ty.name.clone(),
            },
            span: SourceSpan::new(field.span.start, field.span.end),
        });
    }
    if field.role != StructFieldRole::Ins {
        return Ok(());
    }
    Err(SemanticError {
        kind: SemanticErrorKind::InvalidStructFieldRole {
            struct_name: struct_name.to_owned(),
            field: field.name.clone(),
            role: "ins".to_owned(),
        },
        span: SourceSpan::new(field.span.start, field.span.end),
    })
}

pub(super) fn ensure_field_mutable(
    struct_name: &str,
    field: &StructField,
    span: SourceSpan,
) -> Result<(), SemanticError> {
    if field.role != StructFieldRole::Abs {
        return Ok(());
    }
    Err(SemanticError {
        kind: SemanticErrorKind::FrozenStructField {
            struct_name: struct_name.to_owned(),
            field: field.name.clone(),
        },
        span,
    })
}
