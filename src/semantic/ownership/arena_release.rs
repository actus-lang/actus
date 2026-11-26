use std::collections::HashSet;

use crate::ast::Role;
use crate::lexer::SourceSpan;

use crate::semantic::analyzer::Analyzer;
use crate::semantic::errors::{SemanticError, SemanticErrorKind};

impl Analyzer {
    /// Rejects moving, dropping, or overwriting an arena while a view derived
    /// from it (an `abs` or `ins` binding) is still live (ADR-0030).
    ///
    /// A view is live while its block is open, so it ends with its block as for
    /// E1011. Placed `erg` values do not block the release; they are rejected
    /// when they are read afterwards (`ensure_arena_alive`). Bindings that are
    /// not arena owners are ignored.
    pub(in crate::semantic) fn ensure_arena_references_inactive(
        &self,
        arena_index: usize,
        arena_name: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(arena_id) = self.binding_arena_provenance.get(&arena_index).copied() else {
            return Ok(());
        };
        if !self.is_arena_owner(arena_index) {
            return Ok(());
        }
        let open_bindings = self.open_binding_indices();
        let live_view = self
            .binding_arena_provenance
            .iter()
            .filter(|(index, provenance)| **provenance == arena_id && **index != arena_index)
            .map(|(index, _)| *index)
            .filter(|index| {
                let binding = &self.model.bindings[*index];
                matches!(binding.role, Role::Abs | Role::Ins)
                    && open_bindings.contains(index)
                    && binding.ownership.is_live()
            })
            .min();
        let Some(view_index) = live_view else { return Ok(()) };
        Err(SemanticError {
            kind: SemanticErrorKind::ArenaReferenceLive {
                arena: arena_name.to_owned(),
                reference: self.model.bindings[view_index].name.clone(),
            },
            span,
        })
    }

    /// Rejects a use of a value placed in an arena once no live owner of that
    /// arena is left in scope, because the arena was moved away or dropped.
    pub(in crate::semantic) fn ensure_arena_alive(
        &self,
        index: usize,
        name: &str,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(arena_id) = self.binding_arena_provenance.get(&index).copied() else {
            return Ok(());
        };
        if self.is_arena_owner(index) {
            return Ok(());
        }
        let owners: Vec<usize> = self
            .binding_arena_provenance
            .iter()
            .filter(|(owner, provenance)| **provenance == arena_id && self.is_arena_owner(**owner))
            .map(|(owner, _)| *owner)
            .collect();
        let open_bindings = self.open_binding_indices();
        let released = !owners.is_empty()
            && !owners.iter().any(|owner| {
                open_bindings.contains(owner) && self.model.bindings[*owner].ownership.is_live()
            });
        let Some(owner) = owners.iter().min().filter(|_| released) else { return Ok(()) };
        Err(SemanticError {
            kind: SemanticErrorKind::ArenaReferenceReleased {
                arena: self.model.bindings[*owner].name.clone(),
                reference: name.to_owned(),
            },
            span,
        })
    }

    fn is_arena_owner(&self, index: usize) -> bool {
        matches!(self.model.bindings[index].role, Role::Erg | Role::Dat)
            && self
                .binding_type_names
                .get(&index)
                .is_some_and(|type_name| type_name.name == "Arena")
    }

    fn open_binding_indices(&self) -> HashSet<usize> {
        self.scopes.iter().flat_map(|scope| scope.bindings.values().copied()).collect()
    }
}
