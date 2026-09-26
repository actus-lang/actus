use super::super::analyzer::Analyzer;
use super::super::errors::SemanticError;
use crate::ast::{EnumPayload, Expr, Pattern, PatternBinding, Role, VariantPayload};

impl Analyzer {
    pub(super) fn bind_pattern_variables(
        &mut self,
        pattern: &Pattern,
        mode: crate::ast::CaseMode,
        subject: &Expr,
    ) -> Result<(), SemanticError> {
        match pattern {
            Pattern::Variant { enum_name, variant, payload, .. } => {
                let candidate_payload = self.enum_types[enum_name]
                    .variants
                    .iter()
                    .find(|item| item.name == *variant)
                    .unwrap()
                    .payload
                    .clone();
                match (candidate_payload, payload) {
                    (EnumPayload::Tuple(types), VariantPayload::Positional(bindings)) => {
                        for (binding, ty) in bindings.iter().zip(types) {
                            let type_name =
                                self.specialize_pattern_type(enum_name, &ty.name, subject);
                            self.bind_pattern_binding(binding, &type_name, mode)?;
                        }
                    }
                    (EnumPayload::Struct(fields), VariantPayload::Named(patterns)) => {
                        for pattern in patterns {
                            let field =
                                fields.iter().find(|field| field.name == pattern.name).unwrap();
                            let type_name =
                                self.specialize_pattern_type(enum_name, &field.ty.name, subject);
                            self.bind_pattern_binding(&pattern.binding, &type_name, mode)?;
                        }
                    }
                    _ => {}
                }
                Ok(())
            }
            Pattern::Literal { .. } | Pattern::Wildcard { .. } => Ok(()),
        }
    }

    fn specialize_pattern_type(&self, enum_name: &str, type_name: &str, subject: &Expr) -> String {
        let Some(application) = self.resolved_type_name(subject) else {
            return type_name.to_owned();
        };
        let Some(parameter) = self.enum_types[enum_name]
            .generic_parameters
            .iter()
            .position(|parameter| parameter.name == type_name)
        else {
            return type_name.to_owned();
        };
        application
            .arguments
            .get(parameter)
            .map(super::super::analyzer::canonical_type_name)
            .unwrap_or_else(|| type_name.to_owned())
    }

    fn bind_pattern_binding(
        &mut self,
        binding: &PatternBinding,
        type_name: &str,
        mode: crate::ast::CaseMode,
    ) -> Result<(), SemanticError> {
        if binding.name == "_" {
            return Ok(());
        }
        let is_generic_payload = self.enum_types.values().any(|definition| {
            definition.generic_parameters.iter().any(|parameter| parameter.name == type_name)
        });
        if !self.is_generic_parameter(type_name) && !is_generic_payload {
            self.validate_type_name(type_name, binding.span)?;
        }
        self.bind(
            match mode {
                crate::ast::CaseMode::Plain => Role::Abs,
                crate::ast::CaseMode::Abs => Role::Abs,
                crate::ast::CaseMode::Dat => Role::Dat,
            },
            binding.name.clone(),
            crate::ast::lookup_builtin_type(type_name),
            binding.span,
        )?;
        let index = self.binding(&binding.name, binding.span)?;
        if self.struct_types.contains_key(type_name) {
            self.binding_struct_types.insert(index, type_name.to_owned());
        }
        if self.enum_types.contains_key(type_name) {
            self.binding_enum_types.insert(index, type_name.to_owned());
        }
        Ok(())
    }
}
