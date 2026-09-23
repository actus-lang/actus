use std::collections::HashSet;

use crate::ast::{
    Expr, Program, Role, StructDef, StructField, StructFieldInit, TopLevelDecl, TypeName,
    lookup_builtin_type,
};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};
use super::type_substitution::TypeSubstitution;

impl Analyzer {
    pub(super) fn validate_field_assignment(
        &mut self,
        object: &Expr,
        field: &str,
        value: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some((name, object_span)) = root_binding(object) else {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidFieldAssignmentTarget { field: field.to_owned() },
                span,
            });
        };
        let binding_index = self.binding(name, object_span)?;
        if self.model.bindings[binding_index].role != Role::Erg {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidFieldAssignmentTarget { field: field.to_owned() },
                span,
            });
        }
        if let crate::semantic::BindingState::Frozen { borrow_ids } =
            &self.model.bindings[binding_index].state
        {
            return Err(SemanticError {
                kind: SemanticErrorKind::FieldBorrowConflict {
                    owner: name.to_owned(),
                    field: field.to_owned(),
                    borrow_ids: borrow_ids.clone(),
                },
                span,
            });
        }
        self.ensure_mutable(binding_index, name, object_span)?;
        if !matches!(object, Expr::Identifier { .. }) {
            self.visit_expression(object)?;
        }
        let Some(struct_name) = self.expression_struct_type(object) else {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidFieldAssignmentTarget { field: field.to_owned() },
                span,
            });
        };
        let Some(struct_field) = self.struct_field(&struct_name, field).cloned() else {
            return Err(SemanticError {
                kind: SemanticErrorKind::UnknownStructField {
                    struct_name,
                    field: field.to_owned(),
                },
                span,
            });
        };
        self.visit_expression(value)?;
        self.validate_field_value(&struct_name, &struct_field, value, span)
    }

    pub(super) fn register_structs(&mut self, program: &Program) -> Result<(), SemanticError> {
        for declaration in &program.declarations {
            let TopLevelDecl::Struct(definition) = declaration else { continue };
            if self.struct_types.insert(definition.name.clone(), definition.clone()).is_some() {
                return Err(SemanticError {
                    kind: SemanticErrorKind::DuplicateStructName { name: definition.name.clone() },
                    span: definition.span,
                });
            }
        }
        let definitions = self.struct_types.values().cloned().collect::<Vec<_>>();
        for definition in definitions {
            self.validate_struct_definition(&definition)?;
        }
        Ok(())
    }

    fn validate_struct_definition(&mut self, definition: &StructDef) -> Result<(), SemanticError> {
        self.with_generic_scope(&definition.generic_parameters, |analyzer| {
            let mut field_names = HashSet::new();
            for field in &definition.fields {
                if !field_names.insert(field.name.clone()) {
                    return Err(SemanticError {
                        kind: SemanticErrorKind::DuplicateStructField {
                            struct_name: definition.name.clone(),
                            field: field.name.clone(),
                        },
                        span: field.span,
                    });
                }
                analyzer.validate_type_reference(&field.ty)?;
            }
            Ok(())
        })
    }

    pub(super) fn record_struct_binding(
        &mut self,
        name: &str,
        type_name: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if !self.struct_types.contains_key(type_name) {
            return Ok(());
        }
        let index = self.binding(name, span)?;
        self.binding_struct_types.insert(index, type_name.to_owned());
        Ok(())
    }

    pub(super) fn record_initializer_struct_type(
        &mut self,
        name: &str,
        initializer: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(type_name) = self.expression_struct_type(initializer) else { return Ok(()) };
        let index = self.binding(name, span)?;
        self.binding_struct_types.insert(index, type_name);
        Ok(())
    }

    pub(super) fn expression_struct_type(&self, expression: &Expr) -> Option<String> {
        match expression {
            Expr::StructLit { name, .. } if self.struct_types.contains_key(name) => {
                Some(name.clone())
            }
            Expr::Identifier { name, span } => self
                .binding(name, *span)
                .ok()
                .and_then(|index| self.binding_struct_types.get(&index).cloned()),
            Expr::Grouping { expression, .. } | Expr::Borrow { expression, .. } => {
                self.expression_struct_type(expression)
            }
            Expr::FieldAccess { object, field, .. } => self
                .expression_struct_type(object)
                .and_then(|name| self.struct_field(&name, field))
                .and_then(|field| {
                    self.struct_types.contains_key(&field.ty.name).then(|| field.ty.name.clone())
                }),
            _ => None,
        }
    }

    pub(super) fn validate_struct_literal(
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
        let declared_type =
            TypeName { name: name.to_owned(), arguments: type_arguments.to_vec(), span };
        self.validate_type_reference(&declared_type)?;
        let substitution =
            TypeSubstitution::for_type(name, &definition.generic_parameters, type_arguments, span)?;
        self.validate_struct_initializers(name, &definition, &substitution, fields, span)?;
        Ok(())
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
        }
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

    pub(super) fn validate_field_access(
        &self,
        object: &Expr,
        field: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(struct_name) = self.expression_struct_type(object) else {
            return Err(SemanticError {
                kind: SemanticErrorKind::UnknownStructField {
                    struct_name: "<non-struct>".to_owned(),
                    field: field.to_owned(),
                },
                span,
            });
        };
        if self.struct_field(&struct_name, field).is_none() {
            return Err(SemanticError {
                kind: SemanticErrorKind::UnknownStructField {
                    struct_name,
                    field: field.to_owned(),
                },
                span,
            });
        }
        Ok(())
    }

    fn validate_field_value(
        &self,
        struct_name: &str,
        field: &StructField,
        value: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        self.validate_field_value_type(struct_name, field, &field.ty, value, span)
    }

    fn validate_field_value_type(
        &self,
        struct_name: &str,
        field: &StructField,
        expected_type: &TypeName,
        value: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let expected = expected_type.name.as_str();
        let found = self.expression_type_name(value).unwrap_or_else(|| "unknown".to_owned());
        let matches = if let Some(expected_builtin) = lookup_builtin_type(expected) {
            self.expression_type(value) == Some(expected_builtin)
        } else {
            self.expression_struct_type(value).as_deref() == Some(expected)
        };
        if matches {
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::StructFieldTypeMismatch {
                struct_name: struct_name.to_owned(),
                field: field.name.clone(),
                expected: expected.to_owned(),
                found,
            },
            span,
        })
    }

    pub(super) fn struct_field(&self, struct_name: &str, field: &str) -> Option<&StructField> {
        self.struct_types.get(struct_name).and_then(|definition| {
            definition.fields.iter().find(|candidate| candidate.name == field)
        })
    }
}

fn root_binding(expression: &Expr) -> Option<(&str, SourceSpan)> {
    match expression {
        Expr::Identifier { name, span } => Some((name, *span)),
        Expr::FieldAccess { object, .. } => root_binding(object),
        _ => None,
    }
}
