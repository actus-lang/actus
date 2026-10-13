use std::collections::HashMap;

use super::analyzer::Analyzer;
use super::argument_shapes::argument_span;
use super::calls::VerbSignature;
use super::errors::{SemanticError, SemanticErrorKind};
use super::model::ExclusiveLoan;
use super::state::AccessState;
use crate::ast::{Argument, Role};

type PendingExclusiveLoan = (usize, usize, String, usize, crate::lexer::SourceSpan);

impl Analyzer {
    pub(super) fn validate_exclusive_aliases(
        &self,
        arguments: &[Argument],
        parameter_indices: &[usize],
        signature: &VerbSignature,
    ) -> Result<(), SemanticError> {
        let mut roots: HashMap<usize, (String, String)> = HashMap::new();
        for (argument, parameter_index) in arguments.iter().zip(parameter_indices) {
            let role = &signature.params[*parameter_index].1;
            if !matches!(role, Role::Ins | Role::Abs) {
                continue;
            }
            let Some(root) = self.root_binding_index(&argument.expression) else { continue };
            let path = field_loan_path(&argument.expression).unwrap_or_default();
            if let Some((previous_path, previous_role)) = roots.get(&root)
                && paths_overlap(previous_path, &path)
                && (previous_role == "ins" || *role == Role::Ins)
            {
                return Err(SemanticError {
                    kind: SemanticErrorKind::ExclusiveLoanAlias {
                        name: self.model.bindings[root].name.clone(),
                    },
                    span: argument_span(argument),
                });
            }
            roots.insert(root, (path, role_name(role).to_owned()));
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
        let mut loans: Vec<PendingExclusiveLoan> = Vec::new();
        for (argument, parameter_index) in arguments.iter().zip(parameter_indices) {
            if let Some(loan) =
                self.prepare_exclusive_loan(callee, argument, *parameter_index, signature)?
            {
                loans.push(loan);
            }
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
                parameter: signature.params[parameter_index].0.clone(),
                origin_span: span,
            });
        }
        Ok(())
    }

    fn prepare_exclusive_loan(
        &mut self,
        callee: &str,
        argument: &Argument,
        parameter_index: usize,
        signature: &VerbSignature,
    ) -> Result<Option<PendingExclusiveLoan>, SemanticError> {
        if signature.params[parameter_index].1 != Role::Ins {
            return Ok(None);
        }
        let Some(root) = self.root_binding_index(&argument.expression) else { return Ok(None) };
        let field_path = field_loan_path(&argument.expression);
        let field_available = field_path.as_ref().is_some_and(|path| {
            !path.is_empty()
                && self
                    .active_field_loans
                    .iter()
                    .filter(|(owner, _, _)| *owner == root)
                    .all(|(_, active, _)| !paths_overlap(active, path))
        });
        if (!matches!(self.model.bindings[root].access, AccessState::Mutable) && !field_available)
            || !self.model.bindings[root].ownership.is_live()
        {
            return Err(SemanticError {
                kind: SemanticErrorKind::InvalidArgumentRole {
                    callee: callee.to_owned(),
                    parameter: signature.params[parameter_index].0.clone(),
                },
                span: argument_span(argument),
            });
        }
        let loan_id = self.next_loan_id;
        self.next_loan_id += 1;
        let owner = self.model.bindings[root].name.clone();
        if matches!(self.model.bindings[root].access, AccessState::Mutable) {
            self.model.bindings[root].access = AccessState::Suspended { loan_id };
        }
        if let Some(field_path) = field_path.filter(|path| !path.is_empty()) {
            self.active_field_loans.push((root, field_path, self.scopes.len()));
        }
        Ok(Some((root, loan_id, owner, parameter_index, argument_span(argument))))
    }
}

fn field_loan_path(expression: &crate::ast::Expr) -> Option<String> {
    match expression {
        crate::ast::Expr::Identifier { .. } => Some(String::new()),
        crate::ast::Expr::FieldAccess { object, field, .. } => {
            let prefix = field_loan_path(object)?;
            Some(if prefix.is_empty() { field.clone() } else { format!("{prefix}.{field}") })
        }
        _ => None,
    }
}

fn paths_overlap(left: &str, right: &str) -> bool {
    left == right
        || left.strip_prefix(right).is_some_and(|suffix| suffix.starts_with('.'))
        || right.strip_prefix(left).is_some_and(|suffix| suffix.starts_with('.'))
}

fn role_name(role: &Role) -> &'static str {
    match role {
        Role::Abs => "abs",
        Role::Ins => "ins",
        _ => "other",
    }
}
