use crate::ast::{Expr, Role};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};
use super::model::BindingState;

impl Analyzer {
    pub(super) fn initialize_owner(
        &mut self,
        expression: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Expr::Identifier { name, span: identifier_span } = expression else { return Ok(()) };
        let index = self.binding(name, *identifier_span)?;
        if self.model.bindings[index].role == Role::Abs {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidOwnerInitializer { name: name.clone() },
                span,
            });
        }
        match self.model.bindings[index].state {
            BindingState::Active => self.model.bindings[index].state = BindingState::Moved,
            BindingState::Frozen { .. } => {
                return Err(SemanticError {
                    kind: SemanticErrorKind::MoveFrozen {
                        name: name.clone(),
                        borrow_ids: self.blocking_borrow_ids(index),
                    },
                    span,
                });
            }
            BindingState::Moved | BindingState::Dropped => {
                return Err(SemanticError {
                    kind: SemanticErrorKind::UseAfterMove { name: name.clone() },
                    span,
                });
            }
        }
        Ok(())
    }

    pub(super) fn visit_return(&mut self, expression: Option<&Expr>) -> Result<(), SemanticError> {
        let Some(expression) = expression else {
            self.plan_return_unwind();
            return Ok(());
        };
        self.visit_expression(expression)?;
        let Expr::Identifier { name, span } = expression else {
            if matches!(expression, Expr::Borrow { .. }) {
                return Err(SemanticError {
                    kind: SemanticErrorKind::BorrowedReturn { name: "temporary borrow".to_owned() },
                    span: expression_span(expression),
                });
            }
            return Ok(());
        };
        let index = self.binding(name, *span)?;
        if self.model.bindings[index].role == Role::Abs {
            return Err(SemanticError {
                kind: SemanticErrorKind::BorrowedReturn { name: name.clone() },
                span: *span,
            });
        }
        match self.model.bindings[index].state {
            BindingState::Active => self.model.bindings[index].state = BindingState::Moved,
            BindingState::Frozen { .. } => {
                return Err(SemanticError {
                    kind: SemanticErrorKind::MoveFrozen {
                        name: name.clone(),
                        borrow_ids: self.blocking_borrow_ids(index),
                    },
                    span: *span,
                });
            }
            BindingState::Moved | BindingState::Dropped => {}
        }
        self.plan_return_unwind();
        Ok(())
    }

    pub(super) fn ensure_readable(
        &self,
        index: usize,
        name: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        match self.model.bindings[index].state {
            BindingState::Moved => Err(SemanticError {
                kind: SemanticErrorKind::UseAfterMove { name: name.to_owned() },
                span,
            }),
            BindingState::Dropped => Err(SemanticError {
                kind: SemanticErrorKind::UseAfterDrop { name: name.to_owned() },
                span,
            }),
            BindingState::Active | BindingState::Frozen { .. } => Ok(()),
        }
    }

    pub(super) fn ensure_mutable(
        &self,
        index: usize,
        name: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        self.ensure_readable(index, name, span)?;
        if matches!(self.model.bindings[index].state, BindingState::Frozen { .. }) {
            return Err(SemanticError {
                kind: SemanticErrorKind::MoveFrozen {
                    name: name.to_owned(),
                    borrow_ids: self.blocking_borrow_ids(index),
                },
                span,
            });
        }
        Ok(())
    }

    pub(super) fn drop_binding(
        &mut self,
        name: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let index = self.binding(name, span)?;
        let binding = &self.model.bindings[index];
        if binding.role == Role::Abs {
            return Err(SemanticError {
                kind: SemanticErrorKind::DropBorrow { name: name.to_owned() },
                span,
            });
        }
        match binding.state {
            BindingState::Active => self.model.bindings[index].state = BindingState::Dropped,
            BindingState::Frozen { .. } => {
                return Err(SemanticError {
                    kind: SemanticErrorKind::DropFrozen {
                        name: name.to_owned(),
                        borrow_ids: self.blocking_borrow_ids(index),
                    },
                    span,
                });
            }
            BindingState::Moved | BindingState::Dropped => {
                return Err(SemanticError {
                    kind: SemanticErrorKind::DoubleDrop { name: name.to_owned() },
                    span,
                });
            }
        }
        Ok(())
    }
}

impl Analyzer {
    pub(super) fn blocking_borrow_ids(&self, index: usize) -> Vec<usize> {
        match &self.model.bindings[index].state {
            BindingState::Frozen { borrow_ids } => borrow_ids.clone(),
            BindingState::Active | BindingState::Moved | BindingState::Dropped => Vec::new(),
        }
    }
}

fn expression_span(expression: &Expr) -> SourceSpan {
    match expression {
        Expr::Identifier { span, .. }
        | Expr::Integer { span, .. }
        | Expr::StringLiteral { span, .. }
        | Expr::Grouping { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Borrow { span, .. }
        | Expr::Call { span, .. } => *span,
    }
}
