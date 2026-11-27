use crate::ast::{BuiltinType, Role};
use crate::lexer::SourceSpan;

use super::analyzer::{Analyzer, ScopeFrame};
use super::errors::{SemanticError, SemanticErrorKind};
use super::model::Binding;
use super::state::{AccessState, OwnershipState};

impl Analyzer {
    pub(super) fn bind(
        &mut self,
        role: Role,
        name: String,
        ty: Option<BuiltinType>,
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        if self.scopes.last().expect("binding requires a scope").bindings.contains_key(&name) {
            return Err(SemanticError { kind: SemanticErrorKind::DuplicateBinding { name }, span });
        }
        if self.scopes.iter().rev().skip(1).any(|scope| scope.bindings.contains_key(&name)) {
            return Err(SemanticError { kind: SemanticErrorKind::ShadowedBinding { name }, span });
        }
        let index = self.model.bindings.len();
        let scope_depth = self.scopes.len();
        self.model.bindings.push(Binding {
            name: name.clone(),
            role,
            ty,
            span,
            ownership: OwnershipState::Active,
            access: AccessState::Mutable,
        });
        self.scopes.last_mut().expect("binding requires a scope").declaration_indices.push(index);
        self.scopes.last_mut().expect("binding requires a scope").bindings.insert(name, index);
        self.binding_scope_depth.insert(index, scope_depth);
        Ok(())
    }

    pub(super) fn binding(&self, name: &str, span: SourceSpan) -> Result<usize, SemanticError> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.bindings.get(name).or_else(|| scope.retired.get(name)).copied())
            .ok_or_else(|| SemanticError {
                kind: SemanticErrorKind::UndeclaredIdentifier { name: name.to_owned() },
                span,
            })
    }

    pub(super) fn enter_scope(&mut self, span: SourceSpan) {
        self.scopes.push(ScopeFrame {
            span,
            bindings: std::collections::HashMap::new(),
            borrow_ids: Vec::new(),
            declaration_indices: Vec::new(),
            payload_cleanup: Vec::new(),
            retired: std::collections::HashMap::new(),
        });
    }

    /// Closes the scope of one branch block of an `if` (ADR-0125). The owners
    /// declared in the branch are released where the branch ends, by its own
    /// exit list, so a path that never runs the branch never releases them.
    /// Like any scope, the branch's names may be declared again outside it.
    /// They stay resolvable from the enclosing scope, as they were before
    /// branches had scopes of their own, for the type queries that judge the
    /// `if` value after its branches closed; they own nothing there.
    pub(super) fn leave_branch_scope(&mut self) {
        let names = {
            let frame = self.scopes.last().expect("branch scope cannot be empty");
            frame
                .bindings
                .iter()
                .chain(frame.retired.iter())
                .map(|(name, index)| (name.clone(), *index))
                .collect::<Vec<_>>()
        };
        self.leave_scope();
        if let Some(parent) = self.scopes.last_mut() {
            parent.retired.extend(names);
        }
    }

    pub(super) fn register_payload_cleanup(
        &mut self,
        binding_index: usize,
        enum_name: String,
        variant: String,
        field: String,
    ) {
        self.scopes.last_mut().expect("payload cleanup requires a scope").payload_cleanup.push((
            binding_index,
            enum_name,
            variant,
            field,
        ));
    }

    pub(super) fn leave_scope(&mut self) {
        let frame = self.scopes.pop().expect("scope stack cannot be empty");
        let scope_depth = self.scopes.len() + 1;
        let released = self
            .active_field_loans
            .iter()
            .filter(|(_, _, depth)| *depth == scope_depth)
            .map(|(index, _, _)| *index)
            .collect::<Vec<_>>();
        self.active_field_loans.retain(|(_, _, depth)| *depth != scope_depth);
        for index in released {
            if !self.active_field_loans.iter().any(|(owner, _, _)| *owner == index) {
                self.model.bindings[index].access = super::state::AccessState::Mutable;
            }
        }
        // A view ends with its block: the owner it froze is released by its own
        // binding index, never by name, so an owner of the same name in another
        // verb or an outer scope is not mistaken for it.
        for borrow_id in &frame.borrow_ids {
            let Some(owner_index) = self.borrow_owner_indices.get(borrow_id).copied() else {
                continue;
            };
            self.active_borrow_ids.remove(borrow_id);
            let still_borrowed = self.active_borrow_ids.iter().any(|other| {
                self.borrow_owner_indices.get(other).is_some_and(|index| *index == owner_index)
            });
            if !still_borrowed {
                self.model.bindings[owner_index].access = AccessState::Mutable;
            }
        }
        self.model.cleanup_plans.push(super::cleanup::plan_scope_cleanup(
            self.scopes.len(),
            frame.span,
            &frame.declaration_indices,
            &frame.borrow_ids,
            &frame.payload_cleanup,
            &self.model.bindings,
            &self.binding_type_names,
        ));
    }
}
