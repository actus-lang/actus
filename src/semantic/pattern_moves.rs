use crate::ast::{EnumPayload, Expr, Pattern, VariantPayload};

use super::analyzer::Analyzer;
use super::errors::SemanticError;

impl Analyzer {
    pub(super) fn register_unbound_payload_cleanup(
        &mut self,
        subject: &Expr,
        pattern: &Pattern,
    ) -> Result<(), SemanticError> {
        let Expr::Identifier { name, span } = subject else { return Ok(()) };
        let binding_index = self.binding(name, *span)?;
        let Pattern::Variant { enum_name, variant, payload, .. } = pattern else { return Ok(()) };
        let definition = self.enum_types[enum_name].clone();
        let substitution = self.enum_substitution_for_binding(binding_index, &definition);
        let candidate_payload =
            definition.variants.iter().find(|item| item.name == *variant).unwrap().payload.clone();
        match (candidate_payload, payload) {
            (EnumPayload::Tuple(types), VariantPayload::Positional(bindings)) => self
                .register_tuple_cleanup(
                    binding_index,
                    enum_name,
                    variant,
                    types,
                    bindings,
                    substitution.as_ref(),
                ),
            (EnumPayload::Struct(fields), VariantPayload::Named(patterns)) => self
                .register_named_cleanup(
                    binding_index,
                    enum_name,
                    variant,
                    fields,
                    patterns,
                    substitution.as_ref(),
                ),
            _ => {}
        }
        Ok(())
    }

    fn enum_substitution_for_binding(
        &self,
        binding_index: usize,
        definition: &crate::ast::EnumDef,
    ) -> Option<super::type_substitution::TypeSubstitution> {
        let type_name = self.binding_enum_type_applications.get(&binding_index)?;
        super::type_substitution::TypeSubstitution::for_type(
            &definition.name,
            &definition.generic_parameters,
            &type_name.arguments,
            type_name.span,
        )
        .ok()
    }

    fn register_tuple_cleanup(
        &mut self,
        binding_index: usize,
        enum_name: &str,
        variant: &str,
        types: Vec<crate::ast::TypeName>,
        bindings: &[crate::ast::PatternBinding],
        substitution: Option<&super::type_substitution::TypeSubstitution>,
    ) {
        for (index, (binding, ty)) in bindings.iter().zip(types).enumerate() {
            let ty = substitution.map(|substitution| substitution.apply(&ty)).unwrap_or(ty);
            if binding.name == "_" && is_owned_payload_type(self, &ty.name) {
                self.register_payload_cleanup(
                    binding_index,
                    enum_name.to_owned(),
                    variant.to_owned(),
                    index.to_string(),
                );
            }
        }
    }

    fn register_named_cleanup(
        &mut self,
        binding_index: usize,
        enum_name: &str,
        variant: &str,
        fields: Vec<crate::ast::EnumField>,
        patterns: &[crate::ast::NamedPattern],
        substitution: Option<&super::type_substitution::TypeSubstitution>,
    ) {
        for pattern in patterns {
            let Some(field) = fields.iter().find(|field| field.name == pattern.name) else {
                continue;
            };
            let field_type = substitution
                .map(|substitution| substitution.apply(&field.ty))
                .unwrap_or_else(|| field.ty.clone());
            if pattern.binding.name == "_" && is_owned_payload_type(self, &field_type.name) {
                self.register_payload_cleanup(
                    binding_index,
                    enum_name.to_owned(),
                    variant.to_owned(),
                    field.name.clone(),
                );
            }
        }
    }
}

fn is_owned_payload_type(analyzer: &Analyzer, type_name: &str) -> bool {
    match crate::ast::lookup_builtin_type(type_name) {
        Some(crate::ast::BuiltinType::Int | crate::ast::BuiltinType::Bool) => false,
        Some(_) => true,
        None => {
            analyzer.struct_types.contains_key(type_name)
                || analyzer.enum_types.contains_key(type_name)
        }
    }
}
