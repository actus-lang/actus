use crate::ast::{Expr, IfBranch, Stmt};

use super::super::errors::{SemanticError, SemanticErrorKind};
use super::Analyzer;
use crate::semantic::analyzer::validation::expression_span;

/// The ownership and access state of one binding at the end of a branch.
type BranchState = (crate::semantic::state::OwnershipState, crate::semantic::state::AccessState);

impl Analyzer {
    pub(in crate::semantic::analyzer) fn visit_if_expression(
        &mut self,
        expression: &Expr,
    ) -> Result<(), SemanticError> {
        let Expr::If { condition, then_branch, else_branch, .. } = expression else {
            unreachable!("conditional visitor received a non-conditional expression")
        };
        self.visit_consumed_expression(condition)?;
        let condition_type =
            self.expression_type_name(condition).unwrap_or_else(|| "unknown".to_owned());
        if condition_type != "Bool" {
            return Err(SemanticError {
                kind: SemanticErrorKind::ConditionTypeMismatch { found: condition_type },
                span: expression_span(condition),
            });
        }
        self.visit_if_branches(expression, then_branch, else_branch.as_ref())?;
        // A statement `if` has no unterminated tail (the parser requires `;`),
        // so this only rejects the value of an `if` expression.
        self.reject_void_block_value(then_branch)?;
        if let Some(IfBranch::Block(block)) = else_branch {
            self.reject_void_block_value(block)?;
        }
        self.validate_if_result(then_branch, else_branch.as_ref(), expression)?;
        self.model.conditional_facts.push(super::super::model::ConditionalFact {
            span: expression_span(expression),
            condition_span: expression_span(condition),
            condition_type,
            then_type: block_tail_type(self, then_branch),
            else_type: else_branch.as_ref().and_then(|branch| match branch {
                IfBranch::Block(block) => block_tail_type(self, block),
                IfBranch::ElseIf(expression) => self.expression_type_name(expression),
            }),
        });
        Ok(())
    }

    /// Analyses both branches of an `if` from the same starting state and
    /// joins their states. The value of a returned `if` leaves through each
    /// branch, so such a branch takes no part in the join (ADR-0125).
    fn visit_if_branches(
        &mut self,
        expression: &Expr,
        then_branch: &crate::ast::Block,
        else_branch: Option<&IfBranch>,
    ) -> Result<(), SemanticError> {
        let returning = self.returned_arms == Some(expression_span(expression));
        if returning {
            self.returned_arms = None;
        }
        let initial_state = self.snapshot_binding_states();
        let binding_count = initial_state.len();
        let watermark = self.next_borrow_id;
        let then_returns = self.visit_branch_block(then_branch, returning)?;
        if then_returns || branch_diverges(then_branch) {
            self.end_leaving_holder_borrows(watermark);
        }
        let mut arm_views = self.block_tail_view_origins(then_branch);
        let then_state = self.snapshot_binding_prefix(binding_count);
        self.restore_binding_states(&initial_state);
        let watermark = self.next_borrow_id;
        let else_returns = match else_branch {
            Some(IfBranch::Block(block)) => {
                let returns = self.visit_branch_block(block, returning)?;
                arm_views.extend(self.block_tail_view_origins(block));
                returns
            }
            Some(IfBranch::ElseIf(nested)) => {
                if returning {
                    self.returned_arms = Some(expression_span(nested));
                }
                self.visit_consumed_expression(nested)?;
                arm_views.extend(self.value_view_origins(nested));
                returning
            }
            None => false,
        };
        if else_returns || else_branch.is_some_and(|branch| else_branch_diverges(self, branch)) {
            self.end_leaving_holder_borrows(watermark);
        }
        self.record_conditional_views(expression_span(expression), arm_views);
        let else_state = self.snapshot_binding_prefix(binding_count);
        self.join_if_states(
            &initial_state,
            (&then_state, then_returns),
            (&else_state, else_returns),
            then_branch,
            else_branch,
            expression,
        )
    }

    /// The right operand of `&&` and `||` runs on one path only, so it is a
    /// branch: it must leave every owner of a resource in the state the path
    /// that skips it does, as the branches of an `if` must (E1061, ADR-0125);
    /// otherwise no exit could know whether to release it. A binding that
    /// owns no resource is released by no exit, so its state may differ.
    pub(in crate::semantic::analyzer) fn visit_conditional_operand(
        &mut self,
        right: &Expr,
    ) -> Result<(), SemanticError> {
        let skipped = self.snapshot_binding_states();
        self.visit_consumed_expression(right)?;
        for (index, (before, _)) in skipped.iter().enumerate() {
            let binding = &self.model.bindings[index];
            let after = &binding.ownership;
            let holds_resource = binding.ty == Some(crate::ast::BuiltinType::Buffer)
                || self
                    .binding_type_names
                    .get(&index)
                    .is_some_and(|type_name| self.type_holds_resource(type_name));
            if before != after && holds_resource {
                return Err(SemanticError {
                    kind: SemanticErrorKind::BranchStateMismatch {
                        name: binding.name.clone(),
                        expected: format!("{before:?}"),
                        found: format!("{after:?}"),
                    },
                    span: expression_span(right),
                });
            }
        }
        Ok(())
    }

    /// Analyses one branch block of an `if` in a scope of its own (ADR-0125).
    /// The value of the block, its unterminated tail, leaves the branch as
    /// the value of the `if`, so an owner named there is moved out before the
    /// branch releases what it still owns.
    ///
    /// When the `if` is the value of a `return` (`returning`), the branch
    /// leaves the verb where it ends: it gets an exit list of its own, under
    /// the span of the block, unless the verb returns a value and the
    /// branch's value is itself a returned `case` or `if`, whose arms get
    /// theirs. Returns whether the branch left the verb that way, so that it
    /// takes no part in the join after the `if`.
    fn visit_branch_block(
        &mut self,
        block: &crate::ast::Block,
        returning: bool,
    ) -> Result<bool, SemanticError> {
        self.enter_scope(block.span);
        let tail = block.statements.last();
        let nested = returning
            && self.returns_value()
            && match tail {
                Some(Stmt::Expression { expression, .. }) => {
                    self.returned_arms = crate::semantic::ownership::returned_arms_span(expression);
                    self.returned_arms.is_some()
                }
                Some(Stmt::If { span, .. }) => {
                    self.returned_arms = Some(*span);
                    true
                }
                _ => false,
            };
        self.visit_block(block)?;
        if let Some(Stmt::Expression { expression, span }) = tail
            && (returning || span.end == expression_span(expression).end)
        {
            self.consume_value_tail(expression, *span)?;
        }
        let leaves = returning && (nested || !branch_diverges(block));
        if leaves && !nested {
            self.plan_return_unwind(block.span);
        }
        self.leave_branch_scope();
        Ok(leaves)
    }

    fn join_if_states(
        &mut self,
        initial_state: &[(
            crate::semantic::state::OwnershipState,
            crate::semantic::state::AccessState,
        )],
        (then_state, then_returns): (&[BranchState], bool),
        (else_state, else_returns): (&[BranchState], bool),
        then_branch: &crate::ast::Block,
        else_branch: Option<&IfBranch>,
        expression: &Expr,
    ) -> Result<(), SemanticError> {
        let then_diverges = then_returns || branch_diverges(then_branch);
        let else_diverges =
            else_returns || else_branch.is_some_and(|branch| else_branch_diverges(self, branch));
        match (then_diverges, else_diverges) {
            (true, false) => self.restore_binding_states(else_state),
            (false, true) => self.restore_binding_states(then_state),
            (true, true) => self.restore_binding_states(initial_state),
            (false, false) => {
                self.validate_branch_join(
                    &[then_state.to_vec(), else_state.to_vec()],
                    expression_span(expression),
                )?;
                self.restore_binding_states(then_state);
            }
        }
        self.refreeze_holder_borrows();
        Ok(())
    }

    fn validate_if_result(
        &self,
        then_branch: &crate::ast::Block,
        else_branch: Option<&IfBranch>,
        expression: &Expr,
    ) -> Result<(), SemanticError> {
        let Some(else_branch) = else_branch else {
            if self.expected_expression_type.is_some() {
                return Err(self.if_type_mismatch("value", "Void", expression));
            }
            return Ok(());
        };
        let then_type = block_tail_type(self, then_branch);
        let else_type = match else_branch {
            IfBranch::Block(block) => block_tail_type(self, block),
            IfBranch::ElseIf(nested) => self.expression_type_name(nested),
        };
        if then_type == else_type || (then_type.is_none() && else_type.is_none()) {
            return Ok(());
        }
        if then_type.is_none() && branch_diverges(then_branch) {
            return Ok(());
        }
        if else_type.is_none() && else_branch_diverges(self, else_branch) {
            return Ok(());
        }
        Err(self.if_type_mismatch(
            then_type.as_deref().unwrap_or("Void"),
            else_type.as_deref().unwrap_or("Void"),
            expression,
        ))
    }

    fn if_type_mismatch(&self, expected: &str, found: &str, expression: &Expr) -> SemanticError {
        SemanticError {
            kind: SemanticErrorKind::TypeMismatch {
                callee: "if".to_owned(),
                parameter: "branches".to_owned(),
                expected: expected.to_owned(),
                found: found.to_owned(),
            },
            span: expression_span(expression),
        }
    }
}

fn block_tail_type(analyzer: &Analyzer, block: &crate::ast::Block) -> Option<String> {
    let Some(Stmt::Expression { expression, span }) = block.statements.last() else {
        return None;
    };
    (span.end == expression_span(expression).end)
        .then(|| analyzer.expression_type_name(expression))?
}

fn branch_diverges(block: &crate::ast::Block) -> bool {
    block.statements.last().is_some_and(statement_diverges)
}

fn statement_diverges(statement: &Stmt) -> bool {
    match statement {
        Stmt::Return { .. } | Stmt::Break { .. } | Stmt::Continue { .. } => true,
        Stmt::If { then_branch, else_branch, .. } => {
            branch_diverges(then_branch)
                && else_branch.as_ref().is_some_and(|branch| match branch {
                    IfBranch::Block(block) => branch_diverges(block),
                    IfBranch::ElseIf(expression) => {
                        matches!(expression.as_ref(), Expr::If { .. })
                    }
                })
        }
        _ => false,
    }
}

fn else_branch_diverges(analyzer: &Analyzer, branch: &IfBranch) -> bool {
    match branch {
        IfBranch::Block(block) => branch_diverges(block),
        IfBranch::ElseIf(expression) => analyzer.expression_type_name(expression).is_none(),
    }
}
