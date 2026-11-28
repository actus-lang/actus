use super::{
    BackendBlock, BackendBlockId, BackendContractError, BackendExpressionProvider, BodyLowering,
};
use crate::ast::{Block, CaseBody, Expr, Stmt};
use crate::lexer::SourceSpan;

/// Lowers the body of a `case` arm. Without a fallthrough the `case` is the
/// value of a `return`, and each arm leaves the verb with a `return` of its
/// own whose exit list the analyzer planned under the arm's span (ADR-0125):
/// the arm's value is returned, or, in a verb without a result, the arm runs
/// as statements and then returns. An arm without a list of its own leaves
/// through the list of the `return` statement at `span`.
pub(super) fn case_return_block<P: BackendExpressionProvider>(
    lowering: &mut BodyLowering<'_, P>,
    id: BackendBlockId,
    branch: &crate::ast::CaseBranch,
    span: SourceSpan,
    fallthrough: Option<BackendBlockId>,
) -> Result<BackendBlock, BackendContractError> {
    let returns_unit = lowering.returns_unit();
    let exit_span = if lowering.provider.has_return_exit(branch.span) { branch.span } else { span };
    let exit = Stmt::Return { value: None, span: exit_span };
    match &branch.body {
        CaseBody::Expression(expression) => {
            let statements = if fallthrough.is_some() {
                vec![Stmt::Expression { expression: *expression.clone(), span }]
            } else if returns_unit {
                vec![Stmt::Expression { expression: *expression.clone(), span }, exit]
            } else {
                vec![Stmt::Return { value: Some(*expression.clone()), span: exit_span }]
            };
            let block = Block { statements, span: branch.span };
            lowering.lower_block(id, &block, fallthrough)
        }
        CaseBody::Block(block) => {
            if fallthrough.is_some() {
                return lowering.lower_block(id, block, fallthrough);
            }
            let mut statements = block.statements.clone();
            let value = match statements.last() {
                Some(Stmt::Expression { expression, .. }) => Some(expression.clone()),
                Some(Stmt::If { condition, then_branch, else_branch, span }) => Some(Expr::If {
                    condition: Box::new(condition.clone()),
                    then_branch: then_branch.clone(),
                    else_branch: else_branch.clone(),
                    span: *span,
                }),
                _ => None,
            };
            let diverges = matches!(
                statements.last(),
                Some(Stmt::Return { .. } | Stmt::Break { .. } | Stmt::Continue { .. })
            );
            match value {
                Some(expression) if !returns_unit => {
                    statements.pop();
                    statements.push(Stmt::Return { value: Some(expression), span: exit_span });
                }
                _ if returns_unit && !diverges => statements.push(exit),
                _ => {}
            }
            lowering.lower_block(id, &Block { statements, span: block.span }, fallthrough)
        }
    }
}
