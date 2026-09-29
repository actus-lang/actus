use crate::ast::{StructField, StructFieldRole};
use crate::lexer::SourceSpan;

use super::super::errors::{SemanticError, SemanticErrorKind};

pub(super) fn validate_field_role(
    struct_name: &str,
    field: &StructField,
) -> Result<(), SemanticError> {
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
