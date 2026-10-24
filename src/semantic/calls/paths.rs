use crate::ast::Expr;
use crate::lexer::SourceSpan;

use super::super::analyzer::Analyzer;
use super::super::errors::{SemanticError, SemanticErrorKind};

pub(super) fn root_binding(expression: &Expr) -> Option<(&String, SourceSpan)> {
    match expression {
        Expr::Identifier { name, span } => Some((name, *span)),
        Expr::FieldAccess { object, .. } => root_binding(object),
        _ => None,
    }
}

pub(super) fn format_field_path(object: &Expr, field: &str) -> String {
    let prefix = format_expression_path(object);
    if prefix.is_empty() { field.to_owned() } else { format!("{prefix}.{field}") }
}

fn format_expression_path(expression: &Expr) -> String {
    match expression {
        Expr::Identifier { .. } => String::new(),
        Expr::FieldAccess { object, field, .. } => {
            let prefix = format_expression_path(object);
            if prefix.is_empty() { field.clone() } else { format!("{prefix}.{field}") }
        }
        _ => String::new(),
    }
}

pub(super) fn paths_overlap(left: &str, right: &str) -> bool {
    left == right
        || left.strip_prefix(right).is_some_and(|suffix| suffix.starts_with('.'))
        || right.strip_prefix(left).is_some_and(|suffix| suffix.starts_with('.'))
}

impl Analyzer {
    pub(super) fn qualified_call_lookup_name(
        &self,
        callee: &str,
        span: SourceSpan,
    ) -> Result<String, SemanticError> {
        let Some((namespace, member)) = callee.rsplit_once("::") else {
            return Ok(callee.to_owned());
        };
        if self.imported_namespaces.contains_key(namespace) {
            return Ok(member.to_owned());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::UnknownVerb { name: callee.to_owned() },
            span,
        })
    }
}
