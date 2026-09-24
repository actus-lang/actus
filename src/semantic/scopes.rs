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
        Ok(())
    }

    pub(super) fn binding(&self, name: &str, span: SourceSpan) -> Result<usize, SemanticError> {
        self.scopes.iter().rev().find_map(|scope| scope.bindings.get(name).copied()).ok_or_else(
            || SemanticError {
                kind: SemanticErrorKind::UndeclaredIdentifier { name: name.to_owned() },
                span,
            },
        )
    }

    pub(super) fn enter_scope(&mut self, span: SourceSpan) {
        self.scopes.push(ScopeFrame {
            span,
            bindings: std::collections::HashMap::new(),
            borrow_ids: Vec::new(),
            declaration_indices: Vec::new(),
            payload_cleanup: Vec::new(),
        });
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
        for borrow_id in &frame.borrow_ids {
            let Some(record) = self.model.borrows.iter().find(|record| record.id == *borrow_id)
            else {
                continue;
            };
            let owner = record.owner.clone();
            self.active_borrow_ids.remove(borrow_id);
            let still_borrowed =
                self.model.borrows.iter().any(|other| {
                    other.owner == owner && self.active_borrow_ids.contains(&other.id)
                });
            if !still_borrowed
                && let Some(index) =
                    self.model.bindings.iter().position(|binding| binding.name == owner)
            {
                self.model.bindings[index].access = AccessState::Mutable;
            }
        }
        self.model.cleanup_plans.push(super::cleanup::plan_scope_cleanup(
            self.scopes.len(),
            frame.span,
            &frame.declaration_indices,
            &frame.borrow_ids,
            &frame.payload_cleanup,
            &self.model.bindings,
        ));
    }
}
