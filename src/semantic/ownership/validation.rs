use crate::ast::{Expr, Role};
use crate::lexer::SourceSpan;

use crate::semantic::analyzer::Analyzer;
use crate::semantic::errors::{SemanticError, SemanticErrorKind};
use crate::semantic::state::{AccessState, OwnershipState};

impl Analyzer {
    pub(in crate::semantic) fn consume_case_subject(
        &mut self,
        subject: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let (index, name) = self.validate_dat_case_subject(subject, span)?;
        match self.model.bindings[index].ownership {
            OwnershipState::Active => {
                self.model.bindings[index].ownership = OwnershipState::Moved;
                self.ownership_change_spans.insert(index, span);
            }
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

    fn validate_dat_case_subject(
        &self,
        subject: &Expr,
        span: SourceSpan,
    ) -> Result<(usize, String), SemanticError> {
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
        self.ensure_arena_references_inactive(index, name, span)?;
        Ok((index, name.clone()))
    }

    pub(in crate::semantic) fn initialize_owner(
        &mut self,
        expression: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some((index, name)) = self.validate_owner_source(expression, span)? else {
            return Ok(());
        };
        match self.model.bindings[index].ownership {
            OwnershipState::Active => {
                self.model.bindings[index].ownership = OwnershipState::Moved;
                self.ownership_change_spans.insert(index, span);
            }
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

    /// Moves the source of a value that is stored into an owning slot: an
    /// `erg` or move-only struct field, an enum payload, or an `erg` argument.
    /// A field of an owner moves by its field path (ADR-0046 2.2); any other
    /// source is consumed as for an `erg` binding.
    pub(in crate::semantic) fn consume_owned_source(
        &mut self,
        expression: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        self.reject_move_out_of_view(expression, span)?;
        if matches!(expression, Expr::FieldAccess { .. })
            && self.root_binding_index(expression).is_some()
            && self.is_move_only_aggregate(expression)
        {
            return self.move_dat_argument(expression, span);
        }
        self.initialize_owner(expression, span)
    }

    /// Moves an owner out through the tail value of a block, the value of an
    /// `if` branch or a `case` arm (ADR-0125): the value leaves the block, so
    /// the block's scope must not release it. Only an owner whose type moves
    /// when it is stored is moved; a scalar is copied, and a view or a loan
    /// is not an owner.
    pub(in crate::semantic) fn consume_value_tail(
        &mut self,
        expression: &Expr,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        // The tail is the block's value, even when it is written with `;`,
        // so a call there is received, not discarded (bug #128).
        self.keep_tail_value(expression);
        let index = match expression {
            // A field moves by its path only when its type is move-only.
            Expr::FieldAccess { .. } => return self.consume_owned_source(expression, span),
            // An element cannot leave its array by value (bug #126).
            Expr::Index { .. } => return self.reject_move_out_of_view(expression, span),
            Expr::Identifier { name, span } => match self.binding(name, *span) {
                Ok(index) => index,
                Err(_) => return Ok(()),
            },
            _ => return Ok(()),
        };
        let binding = &self.model.bindings[index];
        if !matches!(binding.role, Role::Erg | Role::Dat) {
            return Ok(());
        }
        let moves = binding.ty == Some(crate::ast::BuiltinType::Buffer)
            || self
                .binding_type_names
                .get(&index)
                .is_some_and(|type_name| self.type_moves_on_store(type_name));
        if !moves {
            return Ok(());
        }
        self.consume_owned_source(expression, span)
    }

    /// Records that the assignment at `span` releases the value the owner
    /// `index` held before it stores the new one (ADR-0125), and makes the
    /// owner live again: it owns the value it was just given. Nothing is
    /// released when the assigned expression itself consumed the old value
    /// (`b = grow(b: dat b)`), and a partially moved owner releases only the
    /// fields it still owned.
    pub(in crate::semantic) fn replace_owned_value(&mut self, index: usize, span: SourceSpan) {
        let binding = &self.model.bindings[index];
        if !matches!(binding.role, Role::Erg | Role::Dat) {
            return;
        }
        let holds_resource = binding.ty == Some(crate::ast::BuiltinType::Buffer)
            || self
                .binding_type_names
                .get(&index)
                .is_some_and(|type_name| self.type_holds_resource(type_name));
        let moved_fields = match &binding.ownership {
            OwnershipState::Active => Some(Vec::new()),
            OwnershipState::PartiallyMoved { fields } => Some(fields.clone()),
            OwnershipState::Moved | OwnershipState::Dropped => None,
        };
        if let Some(moved_fields) = moved_fields.filter(|_| holds_resource) {
            self.model.replacement_drops.push(crate::semantic::model::ExplicitDrop {
                binding_index: index,
                span,
                moved_fields,
            });
        }
        self.model.bindings[index].ownership = OwnershipState::Active;
    }

    fn validate_owner_source(
        &self,
        expression: &Expr,
        span: SourceSpan,
    ) -> Result<Option<(usize, String)>, SemanticError> {
        let Expr::Identifier { name, span: identifier_span } = expression else { return Ok(None) };
        if self.is_constant_name(name) || self.is_const_generic_parameter(name) {
            return Ok(None);
        }
        let index = self.binding(name, *identifier_span)?;
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
        self.ensure_arena_references_inactive(index, name, span)?;
        Ok(Some((index, name.clone())))
    }

    pub(in crate::semantic) fn ensure_readable(
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
            OwnershipState::Active => self.ensure_arena_alive(index, name, span),
        }
    }

    pub(in crate::semantic) fn ensure_mutable(
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
                kind: SemanticErrorKind::MutateFrozen {
                    name: name.to_owned(),
                    borrow_ids: self.blocking_borrow_ids(index),
                },
                span,
            });
        }
        Ok(())
    }

    pub(in crate::semantic) fn drop_binding(
        &mut self,
        name: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let index = self.binding(name, span)?;
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
        self.ensure_arena_alive(index, name, span)?;
        self.ensure_arena_references_inactive(index, name, span)?;
        // Dropping a partially moved owner releases only the paths that are
        // still owned (ADR-0016 invariant 3); the moved paths are recorded so
        // that the backends skip exactly those.
        let moved_fields = match &binding.ownership {
            OwnershipState::Active => Vec::new(),
            OwnershipState::PartiallyMoved { fields } => fields.clone(),
            OwnershipState::Moved | OwnershipState::Dropped => {
                return Err(SemanticError {
                    kind: SemanticErrorKind::DoubleDrop { name: name.to_owned() },
                    span,
                });
            }
        };
        self.model.bindings[index].ownership = OwnershipState::Dropped;
        self.ownership_change_spans.insert(index, span);
        self.model.explicit_drops.push(crate::semantic::model::ExplicitDrop {
            binding_index: index,
            span,
            moved_fields,
        });
        Ok(())
    }
}

impl Analyzer {
    pub(in crate::semantic) fn blocking_borrow_ids(&self, index: usize) -> Vec<usize> {
        match &self.model.bindings[index].access {
            AccessState::Frozen { borrow_ids } => borrow_ids.clone(),
            AccessState::Mutable => Vec::new(),
            AccessState::Suspended { loan_id } => vec![*loan_id],
        }
    }
}
