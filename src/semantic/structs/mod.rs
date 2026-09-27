use std::collections::HashSet;

use crate::ast::{Expr, Program, Role, StructDef, StructField, TopLevelDecl, TypeName};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};
use super::type_substitution::TypeSubstitution;

mod literals;

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
        self.ensure_access_available(binding_index, name, span)?;
        if !matches!(self.model.bindings[binding_index].role, Role::Erg | Role::Ins) {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidFieldAssignmentTarget { field: field.to_owned() },
                span,
            });
        }
        if let crate::semantic::AccessState::Frozen { borrow_ids } =
            &self.model.bindings[binding_index].access
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
        let Some(struct_name) =
            self.expression_struct_type(object).or_else(|| self.expression_pack_type(object))
        else {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidFieldAssignmentTarget { field: field.to_owned() },
                span,
            });
        };
        if self.pack_field(&struct_name, field).is_some() {
            return self.validate_pack_field_assignment(&struct_name, field, value, span);
        }
        self.validate_struct_field_assignment(&struct_name, field, value, span)?;
        self.record_field_arena_provenance(object, field, value, span)
    }

    fn validate_struct_field_assignment(
        &mut self,
        struct_name: &str,
        field: &str,
        value: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(struct_field) = self.struct_field(struct_name, field).cloned() else {
            return Err(SemanticError {
                kind: SemanticErrorKind::UnknownStructField {
                    struct_name: struct_name.to_owned(),
                    field: field.to_owned(),
                },
                span,
            });
        };
        self.visit_expression(value)?;
        self.validate_field_value(struct_name, &struct_field, value, span)
    }

    fn validate_pack_field_assignment(
        &mut self,
        pack: &str,
        field: &str,
        value: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let field_definition =
            self.pack_field(pack, field).cloned().ok_or_else(|| SemanticError {
                kind: SemanticErrorKind::UnknownStructField {
                    struct_name: pack.to_owned(),
                    field: field.to_owned(),
                },
                span,
            })?;
        if !matches!(field_definition.role, Role::Erg) {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidFieldAssignmentTarget { field: field.to_owned() },
                span,
            });
        }
        self.visit_expression(value)?;
        self.validate_expected_literal(value, &field_definition.ty)
    }

    fn record_field_arena_provenance(
        &mut self,
        object: &Expr,
        field: &str,
        value: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some((name, object_span)) = root_binding(object) else { return Ok(()) };
        let binding_index = self.binding(name, object_span)?;
        self.validate_arena_provenance_target(binding_index, name, value, span)?;
        self.field_arena_provenance.remove(&(binding_index, field.to_owned()));
        let arenas = self.arena_provenances(value);
        if arenas.len() > 1 {
            return Err(SemanticError {
                kind: SemanticErrorKind::CrossArenaReference { name: field.to_owned() },
                span,
            });
        }
        if let Some(arena_id) = arenas.into_iter().next() {
            self.field_arena_provenance.insert((binding_index, field.to_owned()), arena_id);
        }
        Ok(())
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
        type_name: &TypeName,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if !self.struct_types.contains_key(&type_name.name) {
            return Ok(());
        }
        let index = self.binding(name, span)?;
        self.binding_struct_types.insert(index, type_name.name.clone());
        if !type_name.arguments.is_empty() {
            self.binding_struct_type_applications.insert(index, type_name.clone());
        }
        Ok(())
    }

    pub(super) fn record_initializer_struct_type(
        &mut self,
        name: &str,
        initializer: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(type_name) = self.resolved_type_name(initializer) else { return Ok(()) };
        if !self.struct_types.contains_key(&type_name.name) {
            return Ok(());
        }
        let index = self.binding(name, span)?;
        self.binding_struct_types.insert(index, type_name.name.clone());
        if !type_name.arguments.is_empty() {
            self.binding_struct_type_applications.insert(index, type_name);
        }
        Ok(())
    }

    pub(super) fn record_initializer_pack_type(
        &mut self,
        name: &str,
        initializer: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(type_name) = self.resolved_type_name(initializer) else { return Ok(()) };
        if !self.pack_types.contains_key(&type_name.name) {
            return Ok(());
        }
        let index = self.binding(name, span)?;
        self.binding_type_names.insert(index, type_name);
        Ok(())
    }

    pub(super) fn expression_struct_type(&self, expression: &Expr) -> Option<String> {
        self.resolved_type_name(expression)
            .filter(|type_name| self.struct_types.contains_key(&type_name.name))
            .map(|type_name| canonical_type_name(&type_name))
    }

    pub(super) fn expression_pack_type(&self, expression: &Expr) -> Option<String> {
        self.resolved_type_name(expression)
            .filter(|type_name| self.pack_types.contains_key(&type_name.name))
            .map(|type_name| canonical_type_name(&type_name))
    }

    pub(super) fn resolved_type_name(&self, expression: &Expr) -> Option<TypeName> {
        if let Expr::Call { span, .. } | Expr::MethodCall { span, .. } = expression
            && let Some(type_name) = self.inferred_expression_types.get(&(span.start, span.end))
        {
            return Some(type_name.clone());
        }
        match expression {
            Expr::StructLit { name, type_arguments, span, .. } => Some(TypeName {
                name: name.clone(),
                arguments: type_arguments.clone(),
                span: *span,
            }),
            Expr::Identifier { name, span } => self.binding(name, *span).ok().and_then(|index| {
                self.binding_type_names
                    .get(&index)
                    .cloned()
                    .or_else(|| self.binding_enum_type_applications.get(&index).cloned())
                    .or_else(|| self.binding_struct_type_applications.get(&index).cloned())
                    .or_else(|| {
                        self.binding_struct_types.get(&index).map(|name| TypeName {
                            name: name.clone(),
                            arguments: Vec::new(),
                            span: *span,
                        })
                    })
            }),
            Expr::Grouping { expression, .. } | Expr::Borrow { expression, .. } => {
                self.resolved_type_name(expression)
            }
            Expr::Try { expression, .. } => self
                .enum_type_application(expression)
                .and_then(|result| result.arguments.first().cloned()),
            Expr::FieldAccess { object, field, .. } => self
                .resolved_type_name(object)
                .and_then(|type_name| self.specialized_field_type(&type_name, field)),
            _ => None,
        }
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

    pub(super) fn validate_field_access(
        &self,
        object: &Expr,
        field: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(struct_name) =
            self.expression_struct_type(object).or_else(|| self.expression_pack_type(object))
        else {
            return Err(SemanticError {
                kind: SemanticErrorKind::UnknownStructField {
                    struct_name: "<non-struct>".to_owned(),
                    field: field.to_owned(),
                },
                span,
            });
        };
        if self.struct_field(&struct_name, field).is_none()
            && self.pack_field(&struct_name, field).is_none()
        {
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

    pub(super) fn struct_field(&self, struct_name: &str, field: &str) -> Option<&StructField> {
        let base_name = struct_name.split('[').next().unwrap_or(struct_name);
        self.struct_types.get(base_name).and_then(|definition| {
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

pub(super) fn canonical_type_name(type_name: &TypeName) -> String {
    if type_name.arguments.is_empty() {
        return type_name.name.clone();
    }
    format!(
        "{}[{}]",
        type_name.name,
        type_name.arguments.iter().map(canonical_type_name).collect::<Vec<_>>().join(",")
    )
}
