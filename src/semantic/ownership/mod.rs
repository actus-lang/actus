use crate::ast::{Expr, Role};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};
use super::state::{AccessState, OwnershipState};

mod returns;

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
        self.ensure_access_available(index, name, span)?;
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
        self.ensure_arena_references_inactive(index, name, span)?;
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
        self.ensure_access_available(index, name, span)?;
        if self.model.bindings[index].role == Role::Ins {
            return Err(SemanticError {
                kind: SemanticErrorKind::EscapingLoan { name: name.clone() },
                span,
            });
        }
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

    pub(super) fn ensure_readable(
        &self,
        index: usize,
        name: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        self.ensure_access_available(index, name, span)?;
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

    pub(super) fn ensure_access_available(
        &self,
        index: usize,
        name: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if let AccessState::Suspended { loan_id } = self.model.bindings[index].access {
            return Err(SemanticError {
                kind: SemanticErrorKind::SuspendedAccess { name: name.to_owned(), loan_id },
                span,
            });
        }
        Ok(())
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
        self.ensure_access_available(index, name, span)?;
        let binding = &self.model.bindings[index];
        if matches!(binding.role, Role::Abs | Role::Ins) {
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
        self.ensure_arena_references_inactive(index, name, span)?;
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

    fn ensure_arena_references_inactive(
        &self,
        arena_index: usize,
        arena_name: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(arena_id) = self.binding_arena_provenance.get(&arena_index).copied() else {
            return Ok(());
        };
        if let Some((reference_index, _)) = self
            .binding_arena_provenance
            .iter()
            .filter(|(index, provenance)| **provenance == arena_id && **index != arena_index)
            .find(|(index, _)| {
                let binding = &self.model.bindings[**index];
                matches!(binding.role, Role::Abs | Role::Ins) && binding.ownership.is_live()
            })
        {
            return Err(SemanticError {
                kind: SemanticErrorKind::ArenaReferenceLive {
                    arena: arena_name.to_owned(),
                    reference: self.model.bindings[*reference_index].name.clone(),
                },
                span,
            });
        }
        Ok(())
    }
}

impl Analyzer {
    pub(super) fn blocking_borrow_ids(&self, index: usize) -> Vec<usize> {
        match &self.model.bindings[index].access {
            AccessState::Frozen { borrow_ids } => borrow_ids.clone(),
            AccessState::Mutable => Vec::new(),
            AccessState::Suspended { loan_id } => vec![*loan_id],
        }
    }
}
