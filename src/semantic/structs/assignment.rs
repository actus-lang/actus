use crate::ast::{Expr, Role};
use crate::lexer::SourceSpan;

use super::super::analyzer::Analyzer;
use super::super::errors::{SemanticError, SemanticErrorKind};

impl Analyzer {
    pub(crate) fn validate_field_assignment(
        &mut self,
        object: &Expr,
        field: &str,
        value: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let struct_name = self.validate_assignment_target(object, field, span)?;
        if self.pack_field(&struct_name, field).is_some() {
            return self.validate_pack_field_assignment(&struct_name, field, value, span);
        }
        self.ensure_struct_field_mutable(&struct_name, field, span)?;
        self.validate_struct_field_assignment(&struct_name, field, value, span)?;
        self.record_field_arena_provenance(object, field, value, span)
    }

    fn validate_assignment_target(
        &mut self,
        object: &Expr,
        field: &str,
        span: SourceSpan,
    ) -> Result<String, SemanticError> {
        let Some((name, object_span)) = super::access::root_binding(object) else {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidFieldAssignmentTarget { field: field.to_owned() },
                span,
            });
        };
        let binding_index = self.binding(name, object_span)?;
        self.validate_assignment_owner(binding_index, name, field, span)?;
        self.ensure_mutable(binding_index, name, object_span)?;
        if !matches!(object, Expr::Identifier { .. }) {
            self.validate_mutable_field_path(object, span)?;
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
        Ok(struct_name)
    }

    fn validate_mutable_field_path(
        &self,
        object: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Expr::FieldAccess { object: parent, field, .. } = object else {
            return Ok(());
        };
        self.validate_mutable_field_path(parent, span)?;
        let Some(parent_type) = self.expression_struct_type(parent) else { return Ok(()) };
        self.ensure_struct_field_mutable(&parent_type, field, span)
    }

    fn ensure_struct_field_mutable(
        &self,
        struct_name: &str,
        field: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(definition) = self.struct_field(struct_name, field) else { return Ok(()) };
        super::roles::ensure_field_mutable(struct_name, definition, span)
    }

    fn validate_assignment_owner(
        &self,
        binding_index: usize,
        name: &str,
        field: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
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
        Ok(())
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
        let Some((name, object_span)) = super::access::root_binding(object) else { return Ok(()) };
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
}
