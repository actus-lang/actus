use std::collections::HashSet;

use crate::ast::{Argument, EnumPayload, Expr, TypeName};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};
use super::type_substitution::TypeSubstitution;

mod support;

pub(super) use support::generic_receiver_type;

impl Analyzer {
    pub(super) fn validate_enum_constructor(
        &mut self,
        receiver: &Expr,
        variant: &str,
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let enum_name =
            self.enum_receiver_name(receiver).expect("enum receiver expected").to_owned();
        let definition = self.enum_types[&enum_name].clone();
        if let Some(receiver_type) = support::receiver_type_name(receiver) {
            self.validate_type_reference(&receiver_type)?;
        }
        let substitution = support::enum_substitution(receiver, &definition)?;
        let Some(candidate) = support::find_variant(&definition, variant).cloned() else {
            return Err(support::unknown_variant(&enum_name, variant, span));
        };
        match &candidate.payload {
            EnumPayload::Unit => {
                self.validate_variant_count(&enum_name, variant, 0, arguments, span)
            }
            EnumPayload::Tuple(types) => self.validate_tuple_constructor(
                &enum_name,
                variant,
                types,
                substitution.as_ref(),
                arguments,
                span,
            ),
            EnumPayload::Struct(fields) => self.validate_named_constructor(
                &enum_name,
                variant,
                fields,
                substitution.as_ref(),
                arguments,
                span,
            ),
        }
    }

    fn validate_tuple_constructor(
        &mut self,
        enum_name: &str,
        variant: &str,
        types: &[TypeName],
        substitution: Option<&TypeSubstitution>,
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        self.validate_variant_count(enum_name, variant, types.len(), arguments, span)?;
        for (index, argument) in arguments.iter().enumerate() {
            if let Some(name) = &argument.name {
                return Err(SemanticError {
                    kind: SemanticErrorKind::EnumVariantArgumentName {
                        enum_name: enum_name.to_owned(),
                        variant: variant.to_owned(),
                        name: name.clone(),
                    },
                    span: support::argument_span(argument),
                });
            }
            let expected = substitution
                .map(|substitution| substitution.apply(&types[index]))
                .unwrap_or_else(|| types[index].clone());
            self.validate_variant_argument(
                enum_name,
                variant,
                &index.to_string(),
                &expected,
                argument,
            )?;
        }
        Ok(())
    }

    fn validate_named_constructor(
        &mut self,
        enum_name: &str,
        variant: &str,
        fields: &[crate::ast::EnumField],
        substitution: Option<&TypeSubstitution>,
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        self.validate_variant_count(enum_name, variant, fields.len(), arguments, span)?;
        let mut seen = HashSet::new();
        for argument in arguments {
            let Some(name) = &argument.name else {
                return Err(SemanticError {
                    kind: SemanticErrorKind::EnumVariantArgumentName {
                        enum_name: enum_name.to_owned(),
                        variant: variant.to_owned(),
                        name: "<positional>".to_owned(),
                    },
                    span: support::argument_span(argument),
                });
            };
            if !seen.insert(name.clone()) {
                return Err(support::named_argument_error(enum_name, variant, name, argument));
            }
            let Some(field) = fields.iter().find(|field| field.name == *name) else {
                return Err(support::named_argument_error(enum_name, variant, name, argument));
            };
            let expected = substitution
                .map(|substitution| substitution.apply(&field.ty))
                .unwrap_or_else(|| field.ty.clone());
            self.validate_variant_argument(enum_name, variant, name, &expected, argument)?;
        }
        Ok(())
    }

    fn validate_variant_count(
        &self,
        enum_name: &str,
        variant: &str,
        expected: usize,
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if expected == arguments.len() {
            return Ok(());
        }
        Err(support::argument_count(enum_name, variant, expected, arguments.len(), span))
    }

    fn validate_variant_argument(
        &mut self,
        _enum_name: &str,
        variant: &str,
        parameter: &str,
        expected: &TypeName,
        argument: &Argument,
    ) -> Result<(), SemanticError> {
        self.visit_expression(&argument.expression)?;
        let found =
            self.expression_type_name(&argument.expression).unwrap_or_else(|| "unknown".to_owned());
        let expected_name = support::canonical_type_name(expected);
        if found == expected_name || found == support::strip_reference_role(&expected_name) {
            if self.enum_payload_owns_value(expected) {
                self.initialize_owner(&argument.expression, support::argument_span(argument))?;
            }
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::EnumVariantArgumentTypeMismatch {
                variant: variant.to_owned(),
                parameter: parameter.to_owned(),
                expected: expected_name,
                found,
            },
            span: support::argument_span(argument),
        })
    }

    pub(super) fn enum_payload_owns_value(&self, type_name: &TypeName) -> bool {
        type_name.name == "Buffer"
            || self.struct_types.contains_key(&type_name.name)
            || self.drop_types.contains(&type_name.name)
    }
}
