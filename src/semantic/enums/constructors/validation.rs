use std::collections::HashSet;

use crate::ast::{Argument, EnumPayload, Expr, PrimitiveType, Role, TypeName, primitive_type};
use crate::lexer::SourceSpan;

use crate::semantic::TypeSubstitution;
use crate::semantic::analyzer::Analyzer;
use crate::semantic::errors::{SemanticError, SemanticErrorKind};

use super::{type_parsing, variants};

impl Analyzer {
    pub(in crate::semantic) fn validate_enum_constructor(
        &mut self,
        receiver: &Expr,
        resolved_type: Option<&TypeName>,
        variant: &str,
        arguments: &[Argument],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let enum_name =
            self.enum_receiver_name(receiver).expect("enum receiver expected").to_owned();
        let definition = self.enum_types[&enum_name].clone();
        let receiver_type = resolved_type.cloned();
        if !definition.generic_parameters.is_empty()
            && let Some(receiver_type) = &receiver_type
            && !receiver_type.arguments.is_empty()
        {
            self.validate_type_reference(receiver_type)?;
        }
        let substitution = receiver_type
            .as_ref()
            .filter(|_| !definition.generic_parameters.is_empty())
            .map(|type_name| {
                TypeSubstitution::for_type(
                    &definition.name,
                    &definition.generic_parameters,
                    &type_name.arguments,
                    type_name.span,
                )
            })
            .transpose()?;
        let Some(candidate) = variants::find_variant(&definition, variant).cloned() else {
            return Err(variants::unknown_variant(&enum_name, variant, span));
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
                    span: variants::argument_span(argument),
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
                    span: variants::argument_span(argument),
                });
            };
            if !seen.insert(name.clone()) {
                return Err(variants::named_argument_error(enum_name, variant, name, argument));
            }
            let Some(field) = fields.iter().find(|field| field.name == *name) else {
                return Err(variants::named_argument_error(enum_name, variant, name, argument));
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
        Err(variants::argument_count(enum_name, variant, expected, arguments.len(), span))
    }

    fn validate_variant_argument(
        &mut self,
        _enum_name: &str,
        variant: &str,
        parameter: &str,
        expected: &TypeName,
        argument: &Argument,
    ) -> Result<(), SemanticError> {
        // `Result[Void, E].Ok(done())` builds the unit payload from a call that
        // returns nothing, so that call is not a missing value.
        self.drops_next_value = primitive_type(&expected.name) == Some(PrimitiveType::Void);
        self.visit_expression(&argument.expression)?;
        let found = self
            .expression_type_name(&argument.expression)
            .map(|name| self.normalize_imported_type_key(&name))
            .unwrap_or_else(|| "unknown".to_owned());
        let expected_name =
            self.normalize_imported_type_key(&type_parsing::canonical_type_name(expected));
        if found == expected_name || found == variants::strip_reference_role(&expected_name) {
            self.validate_expected_literal(&argument.expression, expected)?;
            self.store_enum_payload(
                expected,
                &argument.expression,
                variants::argument_span(argument),
            )?;
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::EnumVariantArgumentTypeMismatch {
                variant: variant.to_owned(),
                parameter: parameter.to_owned(),
                expected: expected_name,
                found,
            },
            span: variants::argument_span(argument),
        })
    }

    /// Stores a constructor argument in an enum payload. A move-only value
    /// (a buffer, an arena, a struct, or any type that holds a resource)
    /// moves into the enum value (ADR-0046 3, ADR-0059).
    /// An `abs` payload is a read-only view instead: it freezes its source for
    /// the rest of the block, as for an `abs` binding (ADR-0016, ADR-0046 2.3).
    pub(in crate::semantic) fn store_enum_payload(
        &mut self,
        payload_type: &TypeName,
        expression: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if payload_type.reference_role == Some(Role::Abs) {
            return self.register_view_borrow(expression, span);
        }
        let owns = payload_type.reference_role.is_none()
            && (matches!(payload_type.name.as_str(), "Buffer")
                || self.struct_types.contains_key(&payload_type.name)
                || self.drop_types.contains(&payload_type.name)
                || self.type_holds_resource(payload_type));
        if owns {
            return self.consume_owned_source(expression, span);
        }
        Ok(())
    }
}
