// Structured control-flow lowering for the body backend.

use super::{BackendBlock, BackendBlockId, BackendContractError, BackendInstruction, BodyLowering};
use crate::ast::{Block, Expr, Stmt};
use crate::lexer::SourceSpan;

impl<P: super::BackendExpressionProvider> BodyLowering<'_, P> {
    pub(super) fn lower_structured_statement(
        &mut self,
        id: BackendBlockId,
        block: &Block,
        index: usize,
        statement: &Stmt,
        fallthrough: Option<BackendBlockId>,
        instructions: Vec<BackendInstruction>,
    ) -> Result<Option<BackendBlock>, BackendContractError> {
        match statement {
            Stmt::Loop(nested) => self
                .lower_loop_statement(id, block, index, nested, fallthrough, instructions)
                .map(Some),
            Stmt::ForRange { .. } | Stmt::ForArray { .. } => self
                .lower_for_statement(id, block, index, statement, fallthrough, instructions)
                .map(Some),
            Stmt::Block(nested) => self
                .lower_nested_block_statement(id, block, index, nested, fallthrough, instructions)
                .map(Some),
            Stmt::If { .. } => self
                .lower_if_statement(id, block, index, statement, fallthrough, instructions)
                .map(Some),
            Stmt::Return { value: Some(expression @ Expr::Case { .. }), span } => {
                self.lower_case_return(id, expression, *span, instructions).map(Some)
            }
            Stmt::OwnerDecl { initializer: Expr::If { .. }, .. } => self
                .lower_if_owner_statement(id, block, index, statement, fallthrough, instructions)
                .map(Some),
            Stmt::Return { value: Some(expression @ Expr::If { .. }), span } => {
                self.lower_if_return(id, expression, *span, instructions).map(Some)
            }
            Stmt::Expression { expression: Expr::Case { .. }, span } => self
                .lower_case_expression_statement(
                    id,
                    block,
                    index,
                    statement,
                    *span,
                    fallthrough,
                    instructions,
                )
                .map(Some),
            Stmt::OwnerDecl { initializer: Expr::Case { .. }, .. } => self
                .lower_case_owner_statement(id, block, index, statement, fallthrough, instructions)
                .map(Some),
            Stmt::Assignment { value: Expr::Case { .. }, .. } => self
                .lower_case_assignment_statement(
                    id,
                    block,
                    index,
                    statement,
                    fallthrough,
                    instructions,
                )
                .map(Some),
            _ => Ok(None),
        }
    }

    fn lower_if_statement(
        &mut self,
        id: BackendBlockId,
        block: &Block,
        index: usize,
        statement: &Stmt,
        fallthrough: Option<BackendBlockId>,
        mut instructions: Vec<BackendInstruction>,
    ) -> Result<BackendBlock, BackendContractError> {
        let Stmt::If { condition, then_branch, else_branch, span } = statement else {
            unreachable!()
        };
        let continuation_id = self.allocate_block();
        let if_result = self.lower_if(
            condition,
            then_branch,
            else_branch.as_ref(),
            *span,
            continuation_id,
            &mut instructions,
        )?;
        if if_result.terminates {
            return Ok(BackendBlock { parameters: Vec::new(), id, instructions });
        }
        let continuation =
            Block { statements: block.statements[index + 1..].to_vec(), span: block.span };
        let mut continuation_block =
            self.lower_block_in_scope(continuation_id, &continuation, fallthrough)?;
        continuation_block.parameters = if_result.parameters;
        self.blocks.push(continuation_block);
        Ok(BackendBlock { parameters: Vec::new(), id, instructions })
    }

    fn lower_case_owner_statement(
        &mut self,
        id: BackendBlockId,
        block: &Block,
        index: usize,
        statement: &Stmt,
        fallthrough: Option<BackendBlockId>,
        instructions: Vec<BackendInstruction>,
    ) -> Result<BackendBlock, BackendContractError> {
        let Stmt::OwnerDecl { name, ty, initializer: expression, span, .. } = statement else {
            unreachable!()
        };
        self.lower_case_owner(
            id,
            block,
            index,
            name,
            ty.as_deref(),
            expression,
            *span,
            fallthrough,
            instructions,
        )
    }

    fn lower_case_assignment_statement(
        &mut self,
        id: BackendBlockId,
        block: &Block,
        index: usize,
        statement: &Stmt,
        fallthrough: Option<BackendBlockId>,
        instructions: Vec<BackendInstruction>,
    ) -> Result<BackendBlock, BackendContractError> {
        let Stmt::Assignment { target, value: expression, span } = statement else {
            unreachable!()
        };
        let name = format!("__case_value_{}", id.0);
        let mut statements = block.statements.clone();
        statements.insert(
            index + 1,
            Stmt::Assignment {
                target: target.clone(),
                value: Expr::Identifier { name: name.clone(), span: *span },
                span: *span,
            },
        );
        let synthetic = Block { statements, span: block.span };
        self.lower_case_owner(
            id,
            &synthetic,
            index,
            &name,
            None,
            expression,
            *span,
            fallthrough,
            instructions,
        )
    }

    fn lower_case_expression_statement(
        &mut self,
        id: BackendBlockId,
        block: &Block,
        index: usize,
        statement: &Stmt,
        span: SourceSpan,
        fallthrough: Option<BackendBlockId>,
        mut instructions: Vec<BackendInstruction>,
    ) -> Result<BackendBlock, BackendContractError> {
        let continuation_id = self.allocate_block();
        let Stmt::Expression { expression, .. } = statement else { unreachable!() };
        let case_block =
            self.lower_case_statement(id, expression, span, continuation_id, &mut instructions)?;
        let continuation =
            Block { statements: block.statements[index + 1..].to_vec(), span: block.span };
        let continuation_block =
            self.lower_block_in_scope(continuation_id, &continuation, fallthrough)?;
        self.blocks.push(continuation_block);
        Ok(BackendBlock { parameters: Vec::new(), id, instructions: case_block.instructions })
    }

    fn lower_nested_block_statement(
        &mut self,
        id: BackendBlockId,
        block: &Block,
        index: usize,
        nested: &Block,
        fallthrough: Option<BackendBlockId>,
        mut instructions: Vec<BackendInstruction>,
    ) -> Result<BackendBlock, BackendContractError> {
        let nested_id = self.allocate_block();
        let continuation_id = self.allocate_block();
        instructions.push(BackendInstruction::Jump {
            target: nested_id,
            arguments: Vec::new(),
            span: nested.span,
        });
        let nested_block = self.lower_block(nested_id, nested, Some(continuation_id))?;
        let continuation =
            Block { statements: block.statements[index + 1..].to_vec(), span: block.span };
        let continuation_block =
            self.lower_block_in_scope(continuation_id, &continuation, fallthrough)?;
        self.blocks.push(nested_block);
        self.blocks.push(continuation_block);
        Ok(BackendBlock { parameters: Vec::new(), id, instructions })
    }
}
