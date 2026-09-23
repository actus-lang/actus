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
        let candidate_payload = self.enum_types[enum_name]
            .variants
            .iter()
            .find(|item| item.name == *variant)
            .unwrap()
            .payload
            .clone();
        match (candidate_payload, payload) {
            (EnumPayload::Tuple(types), VariantPayload::Positional(bindings)) => {
                for (index, (binding, ty)) in bindings.iter().zip(types).enumerate() {
                    if binding.name == "_" && is_owned_payload_type(self, &ty.name) {
                        self.register_payload_cleanup(
                            binding_index,
                            enum_name.clone(),
                            variant.clone(),
                            index.to_string(),
                        );
                    }
                }
            }
            (EnumPayload::Struct(fields), VariantPayload::Named(patterns)) => {
                for pattern in patterns {
                    let field = fields.iter().find(|field| field.name == pattern.name).unwrap();
                    if pattern.binding.name == "_" && is_owned_payload_type(self, &field.ty.name) {
                        self.register_payload_cleanup(
                            binding_index,
                            enum_name.clone(),
                            variant.clone(),
                            field.name.clone(),
                        );
                    }
                }
            }
            _ => {}
        }
        Ok(())
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
