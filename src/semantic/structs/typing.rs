use crate::ast::{Expr, TypeName};

use super::super::analyzer::Analyzer;
use crate::semantic::TypeSubstitution;

impl Analyzer {
    pub(crate) fn expression_struct_type(&self, expression: &Expr) -> Option<String> {
        self.resolved_type_name(expression).and_then(|type_name| {
            let type_name = self.normalize_imported_type_name(&type_name);
            self.struct_types
                .contains_key(&type_name.name)
                .then(|| super::access::canonical_type_name(&type_name))
        })
    }

    pub(crate) fn expression_pack_type(&self, expression: &Expr) -> Option<String> {
        self.resolved_type_name(expression).and_then(|type_name| {
            let type_name = self.normalize_imported_type_name(&type_name);
            self.pack_types
                .contains_key(&type_name.name)
                .then(|| super::access::canonical_type_name(&type_name))
        })
    }

    pub(super) fn inferred_call_type(&self, expression: &Expr) -> Option<TypeName> {
        let (Expr::Call { span, .. } | Expr::MethodCall { span, .. }) = expression else {
            return None;
        };
        self.inferred_expression_types.get(&self.expression_cache_key(*span)).cloned()
    }

    pub(super) fn identifier_type_name(
        &self,
        name: &str,
        span: crate::lexer::SourceSpan,
    ) -> Option<TypeName> {
        if self.is_const_generic_parameter(name) {
            return Some(TypeName::builtin("Usize", Vec::new(), None, span));
        }
        let index = self.binding(name, span).ok()?;
        self.binding_type_names
            .get(&index)
            .cloned()
            .or_else(|| self.binding_enum_type_applications.get(&index).cloned())
            .or_else(|| self.binding_struct_type_applications.get(&index).cloned())
            .or_else(|| self.binding_struct_type_names.get(&index).cloned())
            .or_else(|| self.binding_struct_types.get(&index).cloned())
    }

    pub(crate) fn specialized_field_type(
        &self,
        type_name: &TypeName,
        field: &str,
    ) -> Option<TypeName> {
        let type_name = self.normalize_imported_type_name(type_name);
        if field == "storage"
            && let Some(pack) = self.pack_types.get(&type_name.name)
        {
            return Some(self.normalize_imported_type_name(pack.storage.type_name()));
        }
        if let Some(pack_field) = self.pack_field(&type_name.name, field) {
            return Some(self.normalize_imported_type_name(&pack_field.ty));
        }
        let definition = self.struct_types.get(&type_name.name)?;
        let field = definition.fields.iter().find(|candidate| candidate.name == field)?;
        if type_name.arguments.is_empty() {
            let mut field_type = field.ty.clone();
            self.normalize_array_capacities(&mut field_type).ok()?;
            return Some(self.normalize_imported_type_name(&field_type));
        }
        TypeSubstitution::for_type(
            &definition.name,
            &definition.generic_parameters,
            &type_name.arguments,
            type_name.span,
        )
        .ok()
        .and_then(|substitution| {
            let mut field_type = substitution.apply(&field.ty);
            self.normalize_array_capacities(&mut field_type).ok()?;
            Some(self.normalize_imported_type_name(&field_type))
        })
    }
}
