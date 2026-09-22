use crate::ast::{Expr, Role};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};
use super::model::{BindingState, BorrowRecord};

impl Analyzer {
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
        if self.model.bindings[owner_index].role != Role::Erg {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidBorrowTarget { name: name.clone() },
                span,
            });
        }
        let field = borrow_field(expression);
        if let Some(field_name) = &field {
            let owner_expression = match expression.as_ref() {
                Expr::FieldAccess { object, .. } => object.as_ref(),
                _ => expression,
            };
            let Some(struct_name) = self.expression_struct_type(owner_expression) else {
                return Err(SemanticError {
                    kind: SemanticErrorKind::InvalidBorrowTarget { name: name.clone() },
                    span,
                });
            };
            if self.struct_field(&struct_name, field_name).is_none() {
                return Err(SemanticError {
                    kind: SemanticErrorKind::UnknownStructField {
                        struct_name,
                        field: field_name.clone(),
                    },
                    span,
                });
            }
        }
        self.ensure_readable(owner_index, name, *owner_span)?;
        self.add_borrow(name, field, owner_index, span);
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
        match &mut self.model.bindings[owner_index].state {
            BindingState::Active => {
                self.model.bindings[owner_index].state =
                    BindingState::Frozen { borrow_ids: vec![borrow_id] }
            }
            BindingState::Frozen { borrow_ids } => borrow_ids.push(borrow_id),
            BindingState::PartiallyMoved { .. } | BindingState::Moved | BindingState::Dropped => {}
        }
    }
}

fn borrow_base(expression: &Expr) -> Option<(&String, &SourceSpan)> {
    match expression {
        Expr::Identifier { name, span } => Some((name, span)),
        Expr::FieldAccess { object, .. } => borrow_base(object),
        _ => None,
    }
}

fn borrow_field(expression: &Expr) -> Option<String> {
    match expression {
        Expr::FieldAccess { field, .. } => Some(field.clone()),
        _ => None,
    }
}
