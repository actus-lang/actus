use std::collections::HashMap;

use super::analyzer::Analyzer;
use super::call_arguments::argument_span;
use super::calls::VerbSignature;
use super::errors::{SemanticError, SemanticErrorKind};
use super::model::ExclusiveLoan;
use super::state::AccessState;
use crate::ast::{Argument, Role};

impl Analyzer {
    pub(super) fn validate_exclusive_aliases(
        &self,
        arguments: &[Argument],
        parameter_indices: &[usize],
        signature: &VerbSignature,
    ) -> Result<(), SemanticError> {
        let mut roots: HashMap<usize, String> = HashMap::new();
        for (argument, parameter_index) in arguments.iter().zip(parameter_indices) {
            let role = &signature.params[*parameter_index].1;
            if !matches!(role, Role::Ins | Role::Abs) {
                continue;
            }
            let Some(root) = self.root_binding_index(&argument.expression) else { continue };
            if let Some(previous) = roots.insert(root, role_name(role).to_owned())
                && (previous == "ins" || *role == Role::Ins)
            {
                return Err(SemanticError {
                    kind: SemanticErrorKind::ExclusiveLoanAlias {
                        name: self.model.bindings[root].name.clone(),
                    },
                    span: argument_span(argument),
                });
            }
        }
        Ok(())
    }

    pub(super) fn execute_exclusive_loans(
        &mut self,
        callee: &str,
        arguments: &[Argument],
        parameter_indices: &[usize],
        signature: &VerbSignature,
    ) -> Result<(), SemanticError> {
        let mut loans = Vec::new();
        for (argument, parameter_index) in arguments.iter().zip(parameter_indices) {
            if signature.params[*parameter_index].1 != Role::Ins {
                continue;
            }
            let Some(root) = self.root_binding_index(&argument.expression) else {
                continue;
            };
            let loan_id = self.next_loan_id;
            self.next_loan_id += 1;
            let owner = self.model.bindings[root].name.clone();
            if !matches!(self.model.bindings[root].access, AccessState::Mutable)
                || !self.model.bindings[root].ownership.is_live()
            {
                return Err(SemanticError {
                    kind: SemanticErrorKind::InvalidArgumentRole {
                        callee: callee.to_owned(),
                        parameter: signature.params[*parameter_index].0.clone(),
                    },
                    span: argument_span(argument),
                });
            }
            self.model.bindings[root].access = AccessState::Suspended { loan_id };
            loans.push((root, loan_id, owner, parameter_index, argument_span(argument)));
        }
        for (root, loan_id, owner, parameter_index, span) in loans {
            let resumed = self.model.bindings[root].access == AccessState::Suspended { loan_id };
            if resumed {
                self.model.bindings[root].access = AccessState::Mutable;
            }
            self.model.exclusive_loans.push(ExclusiveLoan {
                id: loan_id,
                owner,
                callee: callee.to_owned(),
                parameter: signature.params[*parameter_index].0.clone(),
                origin_span: span,
            });
        }
        Ok(())
    }
}

fn role_name(role: &Role) -> &'static str {
    match role {
        Role::Abs => "abs",
        Role::Ins => "ins",
        _ => "other",
    }
}
