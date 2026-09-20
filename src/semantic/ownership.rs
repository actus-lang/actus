use crate::ast::Role;
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};
use super::model::BindingState;

impl Analyzer {
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
                kind: SemanticErrorKind::MoveFrozen { name: name.to_owned() },
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
                    kind: SemanticErrorKind::DropFrozen { name: name.to_owned() },
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
