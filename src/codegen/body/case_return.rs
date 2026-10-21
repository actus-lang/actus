use super::{
    BackendBlock, BackendBlockId, BackendContractError, BackendExpressionProvider, BodyLowering,
};
use crate::ast::{Block, CaseBody, Expr, Stmt};
use crate::lexer::SourceSpan;

pub(super) fn case_return_block<P: BackendExpressionProvider>(
    lowering: &mut BodyLowering<'_, P>,
    id: BackendBlockId,
    branch: &crate::ast::CaseBranch,
    span: SourceSpan,
    fallthrough: Option<BackendBlockId>,
) -> Result<BackendBlock, BackendContractError> {
    match &branch.body {
        CaseBody::Expression(expression) => {
            let statement = if fallthrough.is_some() {
                Stmt::Expression { expression: *expression.clone(), span }
            } else {
                Stmt::Return { value: Some(*expression.clone()), span }
            };
            let block = Block { statements: vec![statement], span: branch.span };
            lowering.lower_block(id, &block, fallthrough)
        }
        CaseBody::Block(block) => {
            if fallthrough.is_none() {
                let value = match block.statements.last() {
                    Some(Stmt::Expression { expression, span }) => {
                        Some((expression.clone(), *span))
                    }
                    Some(Stmt::If { condition, then_branch, else_branch, span }) => Some((
                        Expr::If {
                            condition: Box::new(condition.clone()),
                            then_branch: then_branch.clone(),
                            else_branch: else_branch.clone(),
                            span: *span,
                        },
                        *span,
                    )),
                    _ => None,
                };
                if let Some((expression, expression_span)) = value {
                    let mut statements = block.statements[..block.statements.len() - 1].to_vec();
                    statements
                        .push(Stmt::Return { value: Some(expression), span: expression_span });
                    let block = Block { statements, span: block.span };
                    return lowering.lower_block(id, &block, fallthrough);
                }
            }
            lowering.lower_block(id, block, fallthrough)
        }
    }
}
