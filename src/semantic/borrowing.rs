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
        let Expr::Identifier { name, span: owner_span } = expression.as_ref() else {
            return Ok(());
        };
        let owner_index = self.binding(name, *owner_span)?;
        if self.model.bindings[owner_index].role != Role::Erg {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidBorrowTarget { name: name.clone() },
                span,
            });
        }
        self.add_borrow(name, owner_index, span);
        Ok(())
    }

    fn add_borrow(&mut self, owner: &str, owner_index: usize, span: SourceSpan) {
        let borrow_id = self.next_borrow_id;
        self.next_borrow_id += 1;
        self.model.borrows.push(BorrowRecord {
            id: borrow_id,
            owner: owner.to_owned(),
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
            BindingState::Moved | BindingState::Dropped => {}
        }
    }
}
