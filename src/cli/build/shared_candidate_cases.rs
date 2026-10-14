use crate::ast::{CaseBody, Expr, Pattern};

use super::CandidateContext;

pub(super) fn result_case_expression(expression: &Expr, context: &CandidateContext<'_>) -> bool {
    case_expression(expression, context, &["Ok", "Err"])
}

pub(super) fn scalar_case_expression(expression: &Expr, context: &CandidateContext<'_>) -> bool {
    case_expression(expression, context, &["Ok", "Err", "Some", "None"])
}

fn case_expression(expression: &Expr, context: &CandidateContext<'_>, variants: &[&str]) -> bool {
    let Expr::Case { subject, branches, .. } = expression else { return false };
    if branches.is_empty() || !super::scalar_expression(subject, context) {
        return false;
    }
    branches.iter().enumerate().all(|(index, branch)| {
        if branch.guard.is_some() {
            return false;
        }
        match &branch.body {
            CaseBody::Expression(body) if super::scalar_expression(body, context) => {}
            CaseBody::Block(block) if super::scalar_return_block(block, context) => {}
            _ => return false,
        }
        match &branch.pattern {
            Pattern::Variant { variant, .. } => variants.contains(&variant.as_str()),
            Pattern::Wildcard { .. } => index + 1 == branches.len(),
            Pattern::Literal { .. } => false,
        }
    })
}

pub(super) fn scalar_statement_call(expression: &Expr, context: &CandidateContext<'_>) -> bool {
    let Expr::Call { callee, arguments, .. } = expression else {
        return false;
    };
    context.returns_value.contains_key(callee)
        && arguments.iter().all(|argument| super::scalar_expression(&argument.expression, context))
}
