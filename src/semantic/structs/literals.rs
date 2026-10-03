use std::collections::HashSet;

use crate::ast::{Expr, StructDef, StructField, StructFieldInit, TypeName, lookup_builtin_type};
use crate::lexer::SourceSpan;

use super::super::analyzer::Analyzer;
use super::super::errors::{SemanticError, SemanticErrorKind};
use super::super::type_substitution::TypeSubstitution;
use super::canonical_type_name;

impl Analyzer {
    pub(crate) fn validate_struct_literal(
        &mut self,
        name: &str,
        type_arguments: &[TypeName],
        fields: &[StructFieldInit],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(definition) = self.struct_types.get(name).cloned() else {
            return Err(SemanticError {
                kind: SemanticErrorKind::UnknownType { name: name.to_owned() },
                span,
            });
        };
        let declared_type = TypeName {
            name: name.to_owned(),
            arguments: type_arguments.to_vec(),
            reference_role: None,
            span,
        };
        self.validate_type_reference(&declared_type)?;
        let substitution =
            TypeSubstitution::for_type(name, &definition.generic_parameters, type_arguments, span)?;
        self.validate_struct_initializers(name, &definition, &substitution, fields, span)
    }

    fn validate_struct_initializers(
        &mut self,
        name: &str,
        definition: &StructDef,
        substitution: &TypeSubstitution,
        fields: &[StructFieldInit],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let mut initialized = HashSet::new();
        for initializer in fields {
            if !initialized.insert(initializer.name.clone()) {
                return Err(SemanticError {
                    kind: SemanticErrorKind::DuplicateStructField {
                        struct_name: name.to_owned(),
                        field: initializer.name.clone(),
                    },
                    span: initializer.span,
                });
            }
            self.validate_struct_initializer(name, substitution, initializer)?;
        }
        self.validate_missing_struct_fields(name, definition, &initialized, span)
    }

    fn validate_struct_initializer(
        &mut self,
        name: &str,
        substitution: &TypeSubstitution,
        initializer: &StructFieldInit,
    ) -> Result<(), SemanticError> {
        let Some(field) = self.struct_field(name, &initializer.name).cloned() else {
            return Err(SemanticError {
                kind: SemanticErrorKind::UnknownStructField {
                    struct_name: name.to_owned(),
                    field: initializer.name.clone(),
                },
                span: initializer.span,
            });
        };
        self.visit_expression(&initializer.value)?;
        let expected = substitution.apply(&field.ty);
        self.validate_field_value_type(
            name,
            &field,
            &expected,
            &initializer.value,
            initializer.span,
        )?;
        if matches!(field.role, crate::ast::StructFieldRole::Erg) {
            self.initialize_owner(&initializer.value, initializer.span)?;
        }
        Ok(())
    }

    fn validate_missing_struct_fields(
        &self,
        name: &str,
        definition: &StructDef,
        initialized: &HashSet<String>,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        for field in &definition.fields {
            if !initialized.contains(&field.name) {
                return Err(SemanticError {
                    kind: SemanticErrorKind::MissingStructField {
                        struct_name: name.to_owned(),
                        field: field.name.clone(),
                    },
                    span,
                });
            }
        }
        Ok(())
    }

    pub(crate) fn validate_field_value_type(
        &self,
        struct_name: &str,
        field: &StructField,
        expected_type: &TypeName,
        value: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let expected_name = canonical_type_name(expected_type);
        let expected = expected_type.name.as_str();
        let found = self.expression_type_name(value).unwrap_or_else(|| "unknown".to_owned());
        let matches = if expected_type.arguments.is_empty()
            && let Some(expected_builtin) = lookup_builtin_type(expected)
        {
            self.expression_type(value) == Some(expected_builtin)
        } else {
            self.expression_type_name(value).as_deref() == Some(expected_name.as_str())
        };
        if matches {
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::StructFieldTypeMismatch {
                struct_name: struct_name.to_owned(),
                field: field.name.clone(),
                expected: expected_name,
                found,
            },
            span,
        })
    }
}
