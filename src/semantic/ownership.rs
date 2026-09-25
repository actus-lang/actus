use crate::ast::{Expr, Role};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};
use super::state::{AccessState, OwnershipState};

impl Analyzer {
    pub(super) fn consume_case_subject(
        &mut self,
        subject: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Expr::Identifier { name, span: subject_span } = subject else {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidCaseRole {
                    mode: "dat".to_owned(),
                    subject: "non-binding".to_owned(),
                },
                span,
            });
        };
        let index = self.binding(name, *subject_span)?;
        if self.model.bindings[index].role == Role::Abs {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidCaseRole {
                    mode: "dat".to_owned(),
                    subject: name.clone(),
                },
                span,
            });
        }
        if self.model.bindings[index].access.is_frozen() {
            return Err(SemanticError {
                kind: SemanticErrorKind::MoveFrozen {
                    name: name.clone(),
                    borrow_ids: self.blocking_borrow_ids(index),
                },
                span,
            });
        }
        match self.model.bindings[index].ownership {
            OwnershipState::Active => self.model.bindings[index].ownership = OwnershipState::Moved,
            OwnershipState::PartiallyMoved { .. }
            | OwnershipState::Moved
            | OwnershipState::Dropped => {
                return Err(SemanticError {
                    kind: SemanticErrorKind::UseAfterMove { name: name.clone() },
                    span,
                });
            }
        }
        Ok(())
    }

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
        if self.model.bindings[index].access.is_frozen() {
            return Err(SemanticError {
                kind: SemanticErrorKind::MoveFrozen {
                    name: name.clone(),
                    borrow_ids: self.blocking_borrow_ids(index),
                },
                span,
            });
        }
        match self.model.bindings[index].ownership {
            OwnershipState::Active => self.model.bindings[index].ownership = OwnershipState::Moved,
            OwnershipState::PartiallyMoved { .. }
            | OwnershipState::Moved
            | OwnershipState::Dropped => {
                return Err(SemanticError {
                    kind: SemanticErrorKind::UseAfterMove { name: name.clone() },
                    span,
                });
            }
        }
        Ok(())
    }

    pub(super) fn visit_return(
        &mut self,
        expression: Option<&Expr>,
        statement_span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(expression) = expression else {
            if self.current_return_type.is_some() {
                return Err(SemanticError {
                    kind: SemanticErrorKind::MissingReturnValue,
                    span: statement_span,
                });
            }
            self.plan_return_unwind(statement_span);
            return Ok(());
        };
        self.visit_expression(expression)?;
        self.validate_return_type(expression)?;
        let returned_expression = unwrap_grouping(expression);
        let Expr::Identifier { name, span } = returned_expression else {
            if contains_returned_borrow(expression) {
                return Err(SemanticError {
                    kind: SemanticErrorKind::BorrowedReturn { name: "temporary borrow".to_owned() },
                    span: expression_span(expression),
                });
            }
            self.plan_return_unwind(statement_span);
            return Ok(());
        };
        let index = self.binding(name, *span)?;
        if self.model.bindings[index].role == Role::Abs {
            return Err(SemanticError {
                kind: SemanticErrorKind::BorrowedReturn { name: name.clone() },
                span: *span,
            });
        }
        if self.model.bindings[index].access.is_frozen() {
            return Err(SemanticError {
                kind: SemanticErrorKind::MoveFrozen {
                    name: name.clone(),
                    borrow_ids: self.blocking_borrow_ids(index),
                },
                span: *span,
            });
        }
        match self.model.bindings[index].ownership {
            OwnershipState::Active => self.model.bindings[index].ownership = OwnershipState::Moved,
            OwnershipState::PartiallyMoved { .. }
            | OwnershipState::Moved
            | OwnershipState::Dropped => {}
        }
        self.plan_return_unwind(statement_span);
        Ok(())
    }

    fn validate_return_type(&self, expression: &Expr) -> Result<(), SemanticError> {
        let Some(expected) = self.current_return_type else { return Ok(()) };
        let Some(found) = self.expression_type(expression) else { return Ok(()) };
        if found == expected {
            return Ok(());
        }
        Err(SemanticError {
            kind: SemanticErrorKind::ReturnTypeMismatch {
                expected: expected.spec().name.to_owned(),
                found: found.spec().name.to_owned(),
            },
            span: expression_span(expression),
        })
    }

    pub(super) fn ensure_readable(
        &self,
        index: usize,
        name: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        match self.model.bindings[index].ownership {
            OwnershipState::Moved => Err(SemanticError {
                kind: SemanticErrorKind::UseAfterMove { name: name.to_owned() },
                span,
            }),
            OwnershipState::Dropped => Err(SemanticError {
                kind: SemanticErrorKind::UseAfterDrop { name: name.to_owned() },
                span,
            }),
            OwnershipState::PartiallyMoved { .. } => Err(SemanticError {
                kind: SemanticErrorKind::UseAfterMove { name: name.to_owned() },
                span,
            }),
            OwnershipState::Active => Ok(()),
        }
    }

    pub(super) fn ensure_mutable(
        &self,
        index: usize,
        name: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        self.ensure_readable(index, name, span)?;
        if self.model.bindings[index].role == Role::Abs {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidMutation { name: name.to_owned() },
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
        if binding.access.is_frozen() {
            return Err(SemanticError {
                kind: SemanticErrorKind::DropFrozen {
                    name: name.to_owned(),
                    borrow_ids: self.blocking_borrow_ids(index),
                },
                span,
            });
        }
        match binding.ownership {
            OwnershipState::Active => {
                self.model.bindings[index].ownership = OwnershipState::Dropped
            }
            OwnershipState::PartiallyMoved { .. }
            | OwnershipState::Moved
            | OwnershipState::Dropped => {
                return Err(SemanticError {
                    kind: SemanticErrorKind::DoubleDrop { name: name.to_owned() },
                    span,
                });
            }
        }
        Ok(())
    }
}

fn contains_returned_borrow(expression: &Expr) -> bool {
    match expression {
        Expr::Borrow { .. } => true,
        Expr::Grouping { expression, .. } | Expr::Unary { expression, .. } => {
            contains_returned_borrow(expression)
        }
        Expr::Binary { left, right, .. } => {
            contains_returned_borrow(left) || contains_returned_borrow(right)
        }
        Expr::Call { .. }
        | Expr::MethodCall { .. }
        | Expr::Identifier { .. }
        | Expr::Integer { .. }
        | Expr::BufferLiteral { .. }
        | Expr::FloatLiteral { .. }
        | Expr::StringLiteral { .. } => false,
        Expr::StructLit { .. } | Expr::FieldAccess { .. } => false,
        Expr::Case { .. } => false,
    }
}

fn unwrap_grouping(mut expression: &Expr) -> &Expr {
    while let Expr::Grouping { expression: inner, .. } = expression {
        expression = inner;
    }
    expression
}

impl Analyzer {
    pub(super) fn blocking_borrow_ids(&self, index: usize) -> Vec<usize> {
        match &self.model.bindings[index].access {
            AccessState::Frozen { borrow_ids } => borrow_ids.clone(),
            AccessState::Mutable => Vec::new(),
        }
    }
}

fn expression_span(expression: &Expr) -> SourceSpan {
    match expression {
        Expr::Identifier { span, .. }
        | Expr::Integer { span, .. }
        | Expr::BufferLiteral { span, .. }
        | Expr::FloatLiteral { span, .. }
        | Expr::StringLiteral { span, .. }
        | Expr::Grouping { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Borrow { span, .. }
        | Expr::Call { span, .. }
        | Expr::MethodCall { span, .. }
        | Expr::StructLit { span, .. }
        | Expr::FieldAccess { span, .. }
        | Expr::Case { span, .. } => *span,
    }
}
