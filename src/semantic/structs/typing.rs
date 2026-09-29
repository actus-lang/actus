use crate::ast::{Expr, TypeName};

use super::super::analyzer::Analyzer;
use super::super::type_substitution::TypeSubstitution;

impl Analyzer {
    pub(crate) fn expression_struct_type(&self, expression: &Expr) -> Option<String> {
        self.resolved_type_name(expression)
            .filter(|type_name| self.struct_types.contains_key(&type_name.name))
            .map(|type_name| super::access::canonical_type_name(&type_name))
    }

    pub(crate) fn expression_pack_type(&self, expression: &Expr) -> Option<String> {
        self.resolved_type_name(expression)
            .filter(|type_name| self.pack_types.contains_key(&type_name.name))
            .map(|type_name| super::access::canonical_type_name(&type_name))
    }

    pub(crate) fn resolved_type_name(&self, expression: &Expr) -> Option<TypeName> {
        if let Some(type_name) = self.inferred_call_type(expression) {
            return Some(type_name);
        }
        match expression {
            Expr::StructLit { name, type_arguments, span, .. } => Some(TypeName {
                name: name.clone(),
                arguments: type_arguments.clone(),
                reference_role: None,
                span: *span,
            }),
            Expr::Identifier { name, span } => self.identifier_type_name(name, *span),
            Expr::Grouping { expression, .. } | Expr::Borrow { expression, .. } => {
                self.resolved_type_name(expression)
            }
            Expr::Try { expression, .. } => self
                .enum_type_application(expression)
                .and_then(|result| result.arguments.first().cloned()),
            Expr::FieldAccess { object, field, .. } => self
                .resolved_type_name(object)
                .and_then(|type_name| self.specialized_field_type(&type_name, field)),
            Expr::Index { target, index, .. } => self.validate_index_access(target, index).ok(),
            _ => None,
        }
    }

    fn inferred_call_type(&self, expression: &Expr) -> Option<TypeName> {
        let (Expr::Call { span, .. } | Expr::MethodCall { span, .. }) = expression else {
            return None;
        };
        self.inferred_expression_types.get(&(span.start, span.end)).cloned()
    }

    fn identifier_type_name(&self, name: &str, span: crate::lexer::SourceSpan) -> Option<TypeName> {
        let index = self.binding(name, span).ok()?;
        self.binding_type_names
            .get(&index)
            .cloned()
            .or_else(|| self.binding_enum_type_applications.get(&index).cloned())
            .or_else(|| self.binding_struct_type_applications.get(&index).cloned())
            .or_else(|| {
                self.binding_struct_types.get(&index).map(|name| TypeName {
                    name: name.clone(),
                    arguments: Vec::new(),
                    reference_role: None,
                    span,
                })
            })
    }

    fn specialized_field_type(&self, type_name: &TypeName, field: &str) -> Option<TypeName> {
        if let Some(pack_field) = self.pack_field(&type_name.name, field) {
            return Some(pack_field.ty.clone());
        }
        let definition = self.struct_types.get(&type_name.name)?;
        let field = definition.fields.iter().find(|candidate| candidate.name == field)?;
        if type_name.arguments.is_empty() {
            return Some(field.ty.clone());
        }
        TypeSubstitution::for_type(
            &definition.name,
            &definition.generic_parameters,
            &type_name.arguments,
            type_name.span,
        )
        .ok()
        .map(|substitution| substitution.apply(&field.ty))
    }
}
