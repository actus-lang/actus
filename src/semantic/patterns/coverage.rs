use std::collections::HashSet;

use crate::ast::{CaseBody, CaseBranch, Expr, Stmt, primitive_type};
use crate::lexer::SourceSpan;

use super::analyzer::Analyzer;
use super::diagnostics::{
    duplicate_pattern, is_wildcard, pattern_name, pattern_span, unreachable_pattern,
};
use super::errors::{SemanticError, SemanticErrorKind};
use super::state::{AccessState, OwnershipState};

impl Analyzer {
    pub(in crate::semantic) fn validate_case_patterns(
        &mut self,
        mode: crate::ast::CaseMode,
        subject: &Expr,
        branches: &[CaseBranch],
        span: SourceSpan,
    ) -> Result<(), SemanticError> {
        self.validate_pattern_coverage(subject, branches, span)?;
        let subject_type =
            self.expression_type_name(subject).unwrap_or_else(|| "unknown".to_owned());
        // The value of a returned `case` leaves through each arm (ADR-0125).
        let returning = self.returned_arms == Some(span);
        if returning {
            self.returned_arms = None;
        }
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
            (mode, returning),
            subject,
            branches,
            &subject_type,
            &branch_state,
            branch_count,
            span,
        )?;
        self.validate_branch_join(&branch_results, span)?;
        if let Some(joined_state) = branch_results.first() {
            self.restore_binding_states(joined_state);
        }
        self.leave_scope();
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn validate_case_branches(
        &mut self,
        (mode, returning): (crate::ast::CaseMode, bool),
        subject: &Expr,
        branches: &[CaseBranch],
        subject_type: &str,
        branch_state: &[(OwnershipState, AccessState)],
        branch_count: usize,
        case_span: SourceSpan,
    ) -> Result<Vec<Vec<(OwnershipState, AccessState)>>, SemanticError> {
        let mut branch_results = Vec::new();
        let mut result_type: Option<String> = None;
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
            let leaves = self.visit_case_arm(branch, returning)?;
            if let Some(candidate) = super::analyzer::case_branch_type(self, &branch.body)
                .map(|candidate| self.normalize_imported_type_key(&candidate))
            {
                if let Some(expected) = &result_type
                    && expected != &candidate
                {
                    return Err(SemanticError {
                        kind: SemanticErrorKind::TypeMismatch {
                            callee: "case".to_owned(),
                            parameter: "branches".to_owned(),
                            expected: expected.clone(),
                            found: candidate,
                        },
                        span: branch.span,
                    });
                }
                result_type = Some(candidate);
            }
            self.leave_scope();
            if !leaves && !case_branch_diverges(&branch.body) {
                branch_results.push(self.snapshot_binding_prefix(branch_count));
            }
        }
        if let Some(result_type) = result_type
            && let Some(type_name) = super::calls::parse_type_name_key(&result_type, case_span)
        {
            self.case_result_types.insert(self.expression_cache_key(case_span), type_name);
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

    pub(in crate::semantic) fn snapshot_binding_states(
        &self,
    ) -> Vec<(OwnershipState, AccessState)> {
        self.model
            .bindings
            .iter()
            .map(|binding| (binding.ownership.clone(), binding.access.clone()))
            .collect()
    }

    pub(in crate::semantic) fn restore_binding_states(
        &mut self,
        snapshot: &[(OwnershipState, AccessState)],
    ) {
        for (binding, (ownership, access)) in self.model.bindings.iter_mut().zip(snapshot.iter()) {
            binding.ownership = ownership.clone();
            binding.access = access.clone();
        }
    }

    pub(in crate::semantic) fn snapshot_binding_prefix(
        &self,
        count: usize,
    ) -> Vec<(OwnershipState, AccessState)> {
        self.snapshot_binding_states().into_iter().take(count).collect()
    }

    pub(in crate::semantic) fn validate_branch_join(
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

    /// Analyses the body of one arm inside its scope. The value of the arm
    /// leaves it when the `case` value is used (the `case` is no statement),
    /// so an owner named as that value is moved out before the arm's scope
    /// releases what it owns.
    ///
    /// When the `case` is the value of a `return` (`returning`), the arm
    /// leaves the verb where it ends: it gets an exit list of its own, under
    /// the span of the arm, unless the verb returns a value and the arm's
    /// value is itself a returned `case` or `if`, whose arms get theirs
    /// (ADR-0125). Returns whether the arm left the verb that way, so that it
    /// takes no part in the join.
    fn visit_case_arm(
        &mut self,
        branch: &CaseBranch,
        returning: bool,
    ) -> Result<bool, SemanticError> {
        // A statement `case` discards the value of its arms.
        let value_used = !self.discarded_expression;
        let tail = case_body_tail(&branch.body);
        let nested = returning && self.returns_value() && {
            let tail_if = match &branch.body {
                CaseBody::Block(block) => match block.statements.last() {
                    Some(Stmt::If { span, .. }) => Some(*span),
                    _ => None,
                },
                CaseBody::Expression(_) => None,
            };
            self.returned_arms = tail_if.or_else(|| {
                tail.and_then(|(expression, _)| {
                    crate::semantic::ownership::returned_arms_span(expression)
                })
            });
            self.returned_arms.is_some()
        };
        self.visit_case_body(&branch.body)?;
        if (value_used || returning)
            && let Some((expression, span)) = tail
        {
            self.consume_value_tail(expression, span)?;
        }
        let leaves = returning && (nested || !case_branch_diverges(&branch.body));
        if leaves && !nested {
            self.plan_return_unwind(branch.span);
        }
        Ok(leaves)
    }

    fn visit_case_body(&mut self, body: &CaseBody) -> Result<(), SemanticError> {
        match body {
            CaseBody::Expression(expression) => {
                let expected = self.expected_expression_type.clone();
                self.drops_next_value = self.conditional_value_dropped;
                self.visit_expression_with_expected(expression, expected.as_ref())
            }
            CaseBody::Block(block) => self.visit_block(block),
        }
    }
}

/// The expression whose value an arm produces: an expression arm itself, or
/// the unterminated tail of a block arm.
fn case_body_tail(body: &CaseBody) -> Option<(&Expr, SourceSpan)> {
    match body {
        CaseBody::Expression(expression) => {
            Some((expression.as_ref(), crate::parser::expression_span(expression)))
        }
        CaseBody::Block(block) => match block.statements.last() {
            Some(Stmt::Expression { expression, span }) => Some((expression, *span)),
            _ => None,
        },
    }
}

fn case_branch_diverges(body: &CaseBody) -> bool {
    let CaseBody::Block(block) = body else { return false };
    block.statements.last().is_some_and(|statement| {
        matches!(statement, Stmt::Return { .. } | Stmt::Break { .. } | Stmt::Continue { .. })
    })
}

fn is_plain_case_type(type_name: &str) -> bool {
    matches!(type_name, "Int" | "Bool" | "String" | "Usize") || primitive_type(type_name).is_some()
}
