use super::super::analyzer::Analyzer;
use super::super::errors::{SemanticError, SemanticErrorKind};
use crate::ast::{EnumPayload, Expr, Pattern, PatternBinding, Role, VariantPayload};

impl Analyzer {
    pub(super) fn bind_pattern_variables(
        &mut self,
        pattern: &Pattern,
        mode: crate::ast::CaseMode,
        subject: &Expr,
    ) -> Result<(), SemanticError> {
        match pattern {
            Pattern::Variant { enum_name, variant, payload, span } => {
                self.bind_variant_pattern(enum_name, variant, payload, *span, mode, subject)
            }
            Pattern::Literal { .. } | Pattern::Wildcard { .. } => Ok(()),
        }
    }

    fn bind_variant_pattern(
        &mut self,
        enum_name: &str,
        variant: &str,
        payload: &VariantPayload,
        span: crate::lexer::SourceSpan,
        mode: crate::ast::CaseMode,
        subject: &Expr,
    ) -> Result<(), SemanticError> {
        let Some(definition) = self.enum_types.get(enum_name) else {
            return Err(SemanticError {
                kind: SemanticErrorKind::UnknownType { name: enum_name.to_owned() },
                span,
            });
        };
        let Some(candidate) = definition.variants.iter().find(|item| item.name == variant) else {
            return Err(SemanticError {
                kind: SemanticErrorKind::UnknownEnumVariant {
                    enum_name: enum_name.to_owned(),
                    variant: variant.to_owned(),
                },
                span,
            });
        };
        match (candidate.payload.clone(), payload) {
            (EnumPayload::Tuple(types), VariantPayload::Positional(bindings)) => {
                self.bind_tuple_pattern(enum_name, types, bindings, mode, subject)
            }
            (EnumPayload::Struct(fields), VariantPayload::Named(patterns)) => {
                self.bind_named_pattern(enum_name, variant, fields, patterns, mode, subject)
            }
            _ => Ok(()),
        }
    }

    fn bind_tuple_pattern(
        &mut self,
        enum_name: &str,
        types: Vec<crate::ast::TypeName>,
        bindings: &[crate::ast::PatternBinding],
        mode: crate::ast::CaseMode,
        subject: &Expr,
    ) -> Result<(), SemanticError> {
        for (binding, ty) in bindings.iter().zip(types) {
            let type_name = self.specialize_pattern_type(enum_name, &ty.name, subject)?;
            self.bind_pattern_binding(binding, &type_name, mode)?;
        }
        Ok(())
    }

    fn bind_named_pattern(
        &mut self,
        enum_name: &str,
        variant: &str,
        fields: Vec<crate::ast::EnumField>,
        patterns: &[crate::ast::NamedPattern],
        mode: crate::ast::CaseMode,
        subject: &Expr,
    ) -> Result<(), SemanticError> {
        for pattern in patterns {
            let Some(field) = fields.iter().find(|field| field.name == pattern.name) else {
                return Err(SemanticError {
                    kind: SemanticErrorKind::EnumVariantArgumentName {
                        enum_name: enum_name.to_owned(),
                        variant: variant.to_owned(),
                        name: pattern.name.clone(),
                    },
                    span: pattern.span,
                });
            };
            let type_name = self.specialize_pattern_type(enum_name, &field.ty.name, subject)?;
            self.bind_pattern_binding(&pattern.binding, &type_name, mode)?;
        }
        Ok(())
    }

    fn specialize_pattern_type(
        &self,
        enum_name: &str,
        type_name: &str,
        subject: &Expr,
    ) -> Result<String, SemanticError> {
        let Some(application) = self.resolved_type_name(subject) else {
            return Ok(type_name.to_owned());
        };
        let Some(parameter) = self.enum_types[enum_name]
            .generic_parameters
            .iter()
            .position(|parameter| parameter.name == type_name)
        else {
            return Ok(type_name.to_owned());
        };
        let argument = application.arguments.get(parameter).ok_or_else(|| SemanticError {
            kind: SemanticErrorKind::GenericArityMismatch {
                name: application.name.clone(),
                expected: self.enum_types[enum_name].generic_parameters.len(),
                found: application.arguments.len(),
            },
            span: application.span,
        })?;
        Ok(pattern_type_name(argument))
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
        self.binding_type_names.insert(
            index,
            crate::ast::TypeName {
                name: type_name.to_owned(),
                arguments: Vec::new(),
                reference_role: None,
                span: binding.span,
            },
        );
        if self.struct_types.contains_key(type_name) {
            self.binding_struct_types.insert(index, type_name.to_owned());
        }
        if self.enum_types.contains_key(type_name) {
            self.binding_enum_types.insert(index, type_name.to_owned());
        }
        Ok(())
    }
}

fn pattern_type_name(type_name: &crate::ast::TypeName) -> String {
    let canonical = super::super::analyzer::canonical_type_name(type_name);
    canonical
        .strip_prefix("abs ")
        .or_else(|| canonical.strip_prefix("ins "))
        .unwrap_or(&canonical)
        .to_owned()
}
