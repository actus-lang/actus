use std::collections::HashSet;

use crate::ast::{CaseBody, CaseBranch, Expr};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::errors::{SemanticError, SemanticErrorKind};
use super::pattern_support::{
    duplicate_pattern, is_wildcard, pattern_name, pattern_span, unreachable_pattern,
};
use super::state::{AccessState, OwnershipState};

mod bindings;
mod validation;

impl Analyzer {
    pub(super) fn validate_case_patterns(
        &mut self,
        mode: crate::ast::CaseMode,
        subject: &Expr,
        branches: &[CaseBranch],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        self.validate_pattern_coverage(subject, branches, span)?;
        let subject_type =
            self.expression_type_name(subject).unwrap_or_else(|| "unknown".to_owned());
        self.enter_scope(span);
        match mode {
            crate::ast::CaseMode::Plain if is_plain_case_type(&subject_type) => {}
            crate::ast::CaseMode::Plain => self.borrow_case_subject(subject, span)?,
            crate::ast::CaseMode::Abs => self.borrow_case_subject(subject, span)?,
            crate::ast::CaseMode::Dat => self.consume_case_subject(subject, span)?,
        }
        let branch_state = self.snapshot_binding_states();
        let branch_count = branch_state.len();
        let branch_results = self.validate_case_branches(
            mode,
            subject,
            branches,
            &subject_type,
            &branch_state,
            branch_count,
        )?;
        self.validate_branch_join(&branch_results, span)?;
        if let Some(joined_state) = branch_results.first() {
            self.restore_binding_states(joined_state);
        }
        self.leave_scope();
        Ok(())
    }

    fn validate_case_branches(
        &mut self,
        mode: crate::ast::CaseMode,
        subject: &Expr,
        branches: &[CaseBranch],
        subject_type: &str,
        branch_state: &[(OwnershipState, AccessState)],
        branch_count: usize,
    ) -> Result<Vec<Vec<(OwnershipState, AccessState)>>, SemanticError> {
        let mut branch_results = Vec::new();
        let mut seen = HashSet::new();
        let mut wildcard_seen = false;
        for branch in branches {
            self.restore_binding_states(branch_state);
            self.validate_branch_order(&mut seen, &mut wildcard_seen, branch)?;
            self.validate_pattern(&branch.pattern, subject_type, subject)?;
            self.enter_scope(branch.span);
            self.bind_pattern_variables(&branch.pattern, mode, subject)?;
            if mode == crate::ast::CaseMode::Dat {
                self.register_unbound_payload_cleanup(subject, &branch.pattern)?;
            }
            if let Some(guard) = &branch.guard {
                self.validate_case_guard(guard, branch.span)?;
            }
            self.visit_case_body(&branch.body)?;
            self.leave_scope();
            branch_results.push(self.snapshot_binding_prefix(branch_count));
        }
        Ok(branch_results)
    }

    fn validate_branch_order(
        &self,
        seen: &mut HashSet<String>,
        wildcard_seen: &mut bool,
        branch: &CaseBranch,
    ) -> Result<(), SemanticError> {
        let name = pattern_name(&branch.pattern);
        let guarded = branch.guard.is_some();
        if *wildcard_seen {
            return Err(unreachable_pattern(name, pattern_span(&branch.pattern)));
        }
        if is_wildcard(&branch.pattern) && !guarded {
            *wildcard_seen = true;
        } else if !guarded && !seen.insert(name.clone()) {
            return Err(duplicate_pattern(name, pattern_span(&branch.pattern)));
        }
        Ok(())
    }

    fn snapshot_binding_states(&self) -> Vec<(OwnershipState, AccessState)> {
        self.model
            .bindings
            .iter()
            .map(|binding| (binding.ownership.clone(), binding.access.clone()))
            .collect()
    }

    fn restore_binding_states(&mut self, snapshot: &[(OwnershipState, AccessState)]) {
        for (binding, (ownership, access)) in self.model.bindings.iter_mut().zip(snapshot.iter()) {
            binding.ownership = ownership.clone();
            binding.access = access.clone();
        }
    }

    fn snapshot_binding_prefix(&self, count: usize) -> Vec<(OwnershipState, AccessState)> {
        self.snapshot_binding_states().into_iter().take(count).collect()
    }

    fn validate_branch_join(
        &self,
        branch_results: &[Vec<(OwnershipState, AccessState)>],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        let Some(expected_states) = branch_results.first() else { return Ok(()) };
        for states in branch_results.iter().skip(1) {
            for (index, (expected, found)) in expected_states.iter().zip(states).enumerate() {
                if expected != found {
                    let name = self.model.bindings[index].name.clone();
                    return Err(SemanticError {
                        kind: SemanticErrorKind::BranchStateMismatch {
                            name,
                            expected: format!("{expected:?}"),
                            found: format!("{found:?}"),
                        },
                        span,
                    });
                }
            }
        }
        Ok(())
    }

    fn visit_case_body(&mut self, body: &CaseBody) -> Result<(), SemanticError> {
        match body {
            CaseBody::Expression(expression) => {
                let expected = self.expected_expression_type.clone();
                self.visit_expression_with_expected(expression, expected.as_ref())
            }
            CaseBody::Block(block) => self.visit_block(block),
        }
    }
}

fn is_plain_case_type(type_name: &str) -> bool {
    matches!(type_name, "Int" | "Bool" | "String")
}
