use crate::ast::{Expr, StructField, TypeName};
use crate::lexer::SourceSpan;

use super::super::analyzer::Analyzer;
use super::super::errors::{SemanticError, SemanticErrorKind};

pub(super) fn root_binding(expression: &Expr) -> Option<(&str, SourceSpan)> {
    match expression {
        Expr::Identifier { name, span } => Some((name, *span)),
        Expr::FieldAccess { object, .. } => root_binding(object),
        Expr::Index { target, .. } => root_binding(target),
        _ => None,
    }
}

impl Analyzer {
    pub(crate) fn validate_field_access(
        &self,
        object: &Expr,
        field: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(struct_name) =
            self.expression_struct_type(object).or_else(|| self.expression_pack_type(object))
        else {
            return Err(SemanticError {
                kind: SemanticErrorKind::UnknownStructField {
                    struct_name: "<non-struct>".to_owned(),
                    field: field.to_owned(),
                },
                span,
            });
        };
        let is_pack_storage = field == "storage" && self.pack_types.contains_key(&struct_name);
        if self.struct_field(&struct_name, field).is_none()
            && self.pack_field(&struct_name, field).is_none()
            && !is_pack_storage
        {
            return Err(SemanticError {
                kind: SemanticErrorKind::UnknownStructField {
                    struct_name,
                    field: field.to_owned(),
                },
                span,
            });
        }
        Ok(())
    }

    pub(crate) fn struct_field(&self, struct_name: &str, field: &str) -> Option<&StructField> {
        let base_name = struct_name.split('[').next().unwrap_or(struct_name);
        self.struct_types.get(base_name).and_then(|definition| {
            definition.fields.iter().find(|candidate| candidate.name == field)
        })
    }
}

pub(crate) fn canonical_type_name(type_name: &TypeName) -> String {
    let role = type_name
        .reference_role
        .as_ref()
        .map(|role| match role {
            crate::ast::Role::Abs => "abs ",
            crate::ast::Role::Ins => "ins ",
            crate::ast::Role::Erg => "erg ",
            crate::ast::Role::Dat => "dat ",
        })
        .unwrap_or("");
    if type_name.arguments.is_empty() {
        return format!("{role}{}", type_name.name);
    }
    format!(
        "{role}{}[{}]",
        type_name.name,
        type_name.arguments.iter().map(canonical_type_name).collect::<Vec<_>>().join(",")
    )
}
