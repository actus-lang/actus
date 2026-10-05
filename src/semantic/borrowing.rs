use crate::ast::{Expr, Role};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};
use super::model::{BorrowRecord, Origin};
use super::state::AccessState;

impl Analyzer {
    pub(super) fn borrow_case_subject(
        &mut self,
        subject: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some((name, subject_span)) = borrow_base(subject) else {
            return Err(invalid_case_role("abs", "non-binding", span));
        };
        let index = self.binding(name, *subject_span)?;
        self.ensure_readable(index, name, *subject_span)?;
        if self.model.bindings[index].role != Role::Abs {
            self.add_borrow(name, borrow_field(subject), index, span);
        }
        Ok(())
    }

    pub(super) fn register_borrow(
        &mut self,
        initializer: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Expr::Borrow { expression, .. } = initializer else { return Ok(()) };
        let Some((name, owner_span)) = borrow_base(expression.as_ref()) else {
            return Ok(());
        };
        let owner_index = self.binding(name, *owner_span)?;
        self.ensure_access_available(owner_index, name, span)?;
        let arena_reference = self.binding_arena_provenance.contains_key(&owner_index);
        let field = borrow_field(expression);
        self.validate_borrow_target(
            name,
            owner_index,
            expression,
            field.as_deref(),
            arena_reference,
            span,
        )?;
        self.ensure_readable(owner_index, name, *owner_span)?;
        self.add_borrow(name, field, owner_index, span);
        Ok(())
    }

    fn validate_borrow_target(
        &self,
        name: &str,
        owner_index: usize,
        expression: &Expr,
        field: Option<&str>,
        arena_reference: bool,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        self.validate_borrow_role(owner_index, field.is_some(), arena_reference, name, span)?;
        if let Some(field_name) = field {
            self.validate_borrow_field(expression, field_name, name, span)?;
        }
        Ok(())
    }

    fn validate_borrow_role(
        &self,
        owner_index: usize,
        reference_field: bool,
        arena_reference: bool,
        name: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if !matches!(self.model.bindings[owner_index].role, Role::Erg | Role::Dat)
            && !(arena_reference
                && matches!(self.model.bindings[owner_index].role, Role::Abs | Role::Ins))
            && !(reference_field && self.model.bindings[owner_index].role == Role::Abs)
        {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidBorrowTarget { name: name.to_owned() },
                span,
            });
        }
        Ok(())
    }

    fn validate_borrow_field(
        &self,
        expression: &Expr,
        field_name: &str,
        name: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let owner_expression = match expression {
            Expr::FieldAccess { object, .. } => object.as_ref(),
            _ => expression,
        };
        let Some(struct_name) = self.expression_struct_type(owner_expression) else {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidBorrowTarget { name: name.to_owned() },
                span,
            });
        };
        if self.struct_field(&struct_name, field_name).is_none() {
            return Err(SemanticError {
                kind: SemanticErrorKind::UnknownStructField {
                    struct_name,
                    field: field_name.to_owned(),
                },
                span,
            });
        }
        Ok(())
    }

    pub(super) fn register_origin_borrow(
        &mut self,
        origin: &Origin,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(owner_index) = self.origin_binding_index(origin) else { return Ok(()) };
        let owner = self.model.bindings[owner_index].name.clone();
        if !matches!(self.model.bindings[owner_index].role, Role::Erg | Role::Dat) {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidBorrowTarget { name: owner },
                span,
            });
        }
        self.ensure_readable(owner_index, &owner, span)?;
        self.add_borrow(&owner, None, owner_index, span);
        Ok(())
    }

    fn add_borrow(
        &mut self,
        owner: &str,
        field: Option<String>,
        owner_index: usize,
        span: SourceSpan,
    ) {
        let borrow_id = self.next_borrow_id;
        self.next_borrow_id += 1;
        self.model.borrows.push(BorrowRecord {
            id: borrow_id,
            owner: owner.to_owned(),
            field,
            scope_depth: self.scopes.len(),
            origin_span: span,
        });
        self.active_borrow_ids.insert(borrow_id);
        self.scopes.last_mut().expect("a verb always has a scope").borrow_ids.push(borrow_id);
        match &mut self.model.bindings[owner_index].access {
            AccessState::Mutable => {
                self.model.bindings[owner_index].access =
                    AccessState::Frozen { borrow_ids: vec![borrow_id] }
            }
            AccessState::Frozen { borrow_ids } => borrow_ids.push(borrow_id),
            AccessState::Suspended { .. } => return,
        }
        if !self.model.bindings[owner_index].ownership.is_live() {
            self.model.bindings[owner_index].access = AccessState::Mutable;
        }
    }
}

fn invalid_case_role(mode: &str, subject: &str, span: SourceSpan) -> SemanticError {
    SemanticError {
        kind: SemanticErrorKind::InvalidCaseRole {
            mode: mode.to_owned(),
            subject: subject.to_owned(),
        },
        span,
    }
}

fn borrow_base(expression: &Expr) -> Option<(&String, &SourceSpan)> {
    match expression {
        Expr::Identifier { name, span } => Some((name, span)),
        Expr::FieldAccess { object, .. } => borrow_base(object),
        Expr::Index { target, .. } => borrow_base(target),
        _ => None,
    }
}

fn borrow_field(expression: &Expr) -> Option<String> {
    match expression {
        Expr::FieldAccess { field, .. } => Some(field.clone()),
        _ => None,
    }
}
