use crate::ast::{Expr, Role};
use crate::lexer::SourceSpan;

use super::super::analyzer::Analyzer;
use super::super::errors::{SemanticError, SemanticErrorKind};
use super::super::state::OwnershipState;

impl Analyzer {
    pub(crate) fn move_dat_argument(
        &mut self,
        expression: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if let Expr::FieldAccess { object, field, .. } = expression {
            return self.move_struct_field(object, field, span);
        }
        if matches!(expression, Expr::Call { callee, .. } if matches!(callee.as_str(), "Ok" | "Err"))
        {
            return Ok(());
        }
        let Expr::Identifier { name, span: identifier_span } = expression else {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidDatArgument { name: "expression".to_owned() },
                span,
            });
        };
        self.move_dat_binding(name, *identifier_span, span)
    }

    fn move_dat_binding(
        &mut self,
        name: &str,
        identifier_span: SourceSpan,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let index = self.binding(name, identifier_span)?;
        self.ensure_access_available(index, name, span)?;
        self.reject_ins_dat_transfer(index, name, span)?;
        if self.model.bindings[index].role == Role::Abs {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidDatArgument { name: name.to_owned() },
                span,
            });
        }
        if self.model.bindings[index].access.is_frozen() {
            return Err(SemanticError {
                kind: SemanticErrorKind::MoveFrozen {
                    name: name.to_owned(),
                    borrow_ids: self.blocking_borrow_ids(index),
                },
                span,
            });
        }
        self.update_dat_ownership(index, name, span)
    }

    fn update_dat_ownership(
        &mut self,
        index: usize,
        name: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        match self.model.bindings[index].ownership.clone() {
            OwnershipState::Active => self.model.bindings[index].ownership = OwnershipState::Moved,
            OwnershipState::PartiallyMoved { .. }
            | OwnershipState::Moved
            | OwnershipState::Dropped => {
                return Err(SemanticError {
                    kind: SemanticErrorKind::UseAfterMove { name: name.to_owned() },
                    span,
                });
            }
        }
        Ok(())
    }

    pub(super) fn move_struct_field(
        &mut self,
        object: &Expr,
        field: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some((name, object_span)) = root_binding(object) else {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidDatArgument { name: field.to_owned() },
                span,
            });
        };
        let index = self.binding(name, object_span)?;
        self.ensure_access_available(index, name, span)?;
        self.reject_ins_dat_transfer(index, name, span)?;
        self.validate_struct_field_move(object, field, span)?;
        let field_path = format_field_path(object, field);
        self.apply_struct_field_move(index, name, field_path, span)
    }

    fn apply_struct_field_move(
        &mut self,
        index: usize,
        name: &str,
        field_path: String,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if self.model.bindings[index].access.is_frozen() {
            return Err(SemanticError {
                kind: SemanticErrorKind::FieldBorrowConflict {
                    owner: name.to_owned(),
                    field: field_path,
                    borrow_ids: self.blocking_borrow_ids(index),
                },
                span,
            });
        }
        self.update_struct_field_ownership(index, name, field_path, span)
    }

    fn update_struct_field_ownership(
        &mut self,
        index: usize,
        name: &str,
        field_path: String,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        match self.model.bindings[index].ownership.clone() {
            OwnershipState::Active => {
                self.model.bindings[index].ownership =
                    OwnershipState::PartiallyMoved { fields: vec![field_path] };
                Ok(())
            }
            OwnershipState::PartiallyMoved { fields } => {
                self.append_struct_field_move(index, name, fields, field_path, span)
            }
            OwnershipState::Moved => Err(SemanticError {
                kind: SemanticErrorKind::UseAfterMove { name: name.to_owned() },
                span,
            }),
            OwnershipState::Dropped => Err(SemanticError {
                kind: SemanticErrorKind::UseAfterDrop { name: name.to_owned() },
                span,
            }),
        }
    }

    fn append_struct_field_move(
        &mut self,
        index: usize,
        name: &str,
        fields: Vec<String>,
        field_path: String,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if fields.iter().any(|moved| paths_overlap(moved, &field_path)) {
            return Err(SemanticError {
                kind: SemanticErrorKind::UseAfterMove { name: name.to_owned() },
                span,
            });
        }
        let mut updated = fields;
        updated.push(field_path);
        self.model.bindings[index].ownership = OwnershipState::PartiallyMoved { fields: updated };
        Ok(())
    }

    fn validate_struct_field_move(
        &self,
        object: &Expr,
        field: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(struct_name) = self.expression_struct_type(object) else {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidDatArgument { name: field.to_owned() },
                span,
            });
        };
        let Some(struct_field) = self.struct_field(&struct_name, field) else {
            return Err(SemanticError {
                kind: SemanticErrorKind::UnknownStructField {
                    struct_name,
                    field: field.to_owned(),
                },
                span,
            });
        };
        if matches!(struct_field.role, crate::ast::StructFieldRole::Erg) {
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::InvalidDatArgument { name: field.to_owned() },
            span,
        })
    }

    fn reject_ins_dat_transfer(
        &self,
        index: usize,
        name: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if self.model.bindings[index].role != Role::Ins {
            return Ok(());
        }
        Err(SemanticError { kind: SemanticErrorKind::EscapingLoan { name: name.to_owned() }, span })
    }
}

fn root_binding(expression: &Expr) -> Option<(&String, SourceSpan)> {
    match expression {
        Expr::Identifier { name, span } => Some((name, *span)),
        Expr::FieldAccess { object, .. } => root_binding(object),
        _ => None,
    }
}

fn format_field_path(object: &Expr, field: &str) -> String {
    let prefix = format_expression_path(object);
    if prefix.is_empty() { field.to_owned() } else { format!("{prefix}.{field}") }
}

fn format_expression_path(expression: &Expr) -> String {
    match expression {
        Expr::Identifier { .. } => String::new(),
        Expr::FieldAccess { object, field, .. } => {
            let prefix = format_expression_path(object);
            if prefix.is_empty() { field.clone() } else { format!("{prefix}.{field}") }
        }
        _ => String::new(),
    }
}

fn paths_overlap(left: &str, right: &str) -> bool {
    left == right
        || left.strip_prefix(right).is_some_and(|suffix| suffix.starts_with('.'))
        || right.strip_prefix(left).is_some_and(|suffix| suffix.starts_with('.'))
}
