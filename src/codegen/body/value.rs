use super::super::body_contract::unsupported;
use super::{BackendBlock, BackendBlockId, BackendContractError, BackendInstruction, BodyLowering};
use crate::ast::{Block, Expr, IfBranch, Stmt};
use crate::lexer::SourceSpan;

impl<P: super::BackendExpressionProvider> BodyLowering<'_, P> {
    pub(super) fn lower_if_return(
        &mut self,
        id: BackendBlockId,
        expression: &Expr,
        span: SourceSpan,
        mut instructions: Vec<BackendInstruction>,
    ) -> Result<BackendBlock, BackendContractError> {
        let Expr::If { condition, then_branch, else_branch, .. } = expression else {
            return Err(unsupported(span, "value-producing if expression expected"));
        };
        let condition = self.provider.lower_expression(condition, &mut instructions)?;
        let then_id = self.allocate_block();
        let else_id = self.allocate_block();
        instructions.push(BackendInstruction::Branch {
            condition,
            then_block: then_id,
            else_block: else_id,
            then_arguments: Vec::new(),
            else_arguments: Vec::new(),
            span,
        });
        let then_block = self.return_branch(then_id, then_branch, span)?;
        self.blocks.push(then_block);
        let else_block = match else_branch {
            Some(IfBranch::Block(block)) => self.return_branch(else_id, block, span)?,
            Some(IfBranch::ElseIf(nested)) => {
                self.lower_if_return(else_id, nested, span, Vec::new())?
            }
            None => return Err(unsupported(span, "value-producing if requires an else branch")),
        };
        self.blocks.push(else_block);
        Ok(BackendBlock { parameters: Vec::new(), id, instructions })
    }

    fn return_branch(
        &mut self,
        id: BackendBlockId,
        branch: &Block,
        span: SourceSpan,
    ) -> Result<BackendBlock, BackendContractError> {
        let mut branch = branch.clone();
        let Some(last) = branch.statements.last_mut() else {
            return Err(unsupported(span, "value-producing if branch is empty"));
        };
        if let Stmt::Expression { expression, span: expression_span } = last {
            *last = Stmt::Return { value: Some(expression.clone()), span: *expression_span };
        }
        self.lower_block(id, &branch, None)
    }
}
