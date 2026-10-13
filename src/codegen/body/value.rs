use super::super::body_contract::unsupported;
use super::{
    BackendBlock, BackendBlockId, BackendBlockParameter, BackendContractError, BackendEdgeArgument,
    BackendInstruction, BackendValueId, BodyLowering,
};
use crate::ast::{Block, Expr, IfBranch, Stmt};
use crate::lexer::SourceSpan;

struct IfOwnerInput<'a> {
    id: BackendBlockId,
    block: &'a Block,
    index: usize,
    name: &'a str,
    declared_type: Option<&'a str>,
    span: SourceSpan,
    fallthrough: Option<BackendBlockId>,
    merge_id: BackendBlockId,
    instructions: Vec<BackendInstruction>,
    then_block: BackendBlock,
    then_value: BackendValueId,
    else_block: BackendBlock,
    else_value: BackendValueId,
}

impl<P: super::BackendExpressionProvider> BodyLowering<'_, P> {
    pub(super) fn lower_if_owner_statement(
        &mut self,
        id: BackendBlockId,
        block: &Block,
        index: usize,
        statement: &Stmt,
        fallthrough: Option<BackendBlockId>,
        mut instructions: Vec<BackendInstruction>,
    ) -> Result<BackendBlock, BackendContractError> {
        let Stmt::OwnerDecl { name, ty, initializer: expression, span, .. } = statement else {
            unreachable!()
        };
        let Expr::If { condition, then_branch, else_branch, .. } = expression else {
            return Err(unsupported(*span, "value-producing if expression expected"));
        };
        let condition = self.provider.lower_expression(condition, &mut instructions)?;
        let merge_id = self.allocate_block();
        let then_id = self.allocate_block();
        let else_id = self.allocate_block();
        instructions.push(BackendInstruction::Branch {
            condition,
            then_block: then_id,
            else_block: else_id,
            then_arguments: Vec::new(),
            else_arguments: Vec::new(),
            span: *span,
        });
        let (then_block, then_value) = self.lower_if_value_branch(then_id, then_branch, *span)?;
        let Some(IfBranch::Block(else_branch)) = else_branch else {
            return Err(unsupported(*span, "value-producing if requires a block else branch"));
        };
        let (else_block, else_value) = self.lower_if_value_branch(else_id, else_branch, *span)?;
        self.finish_if_owner(IfOwnerInput {
            id,
            block,
            index,
            name,
            declared_type: ty.as_deref(),
            span: *span,
            fallthrough,
            merge_id,
            instructions,
            then_block,
            then_value,
            else_block,
            else_value,
        })
    }

    fn finish_if_owner(
        &mut self,
        input: IfOwnerInput<'_>,
    ) -> Result<BackendBlock, BackendContractError> {
        let IfOwnerInput {
            id,
            block,
            index,
            name,
            declared_type,
            span,
            fallthrough,
            merge_id,
            instructions,
            mut then_block,
            then_value,
            mut else_block,
            else_value,
        } = input;
        let result_type = self
            .provider
            .value_type(then_value)
            .ok_or_else(|| unsupported(span, "if branch result type is unavailable"))?;
        if self.provider.value_type(else_value) != Some(result_type) {
            return Err(unsupported(span, "if branches have different value types"));
        }
        let result = self.provider.allocate_value(result_type);
        let mut merge_instructions = Vec::new();
        self.provider.bind_owner(name, result, declared_type, span, &mut merge_instructions)?;
        let continuation =
            Block { statements: block.statements[index + 1..].to_vec(), span: block.span };
        let mut continuation_block =
            self.lower_block_in_scope(merge_id, &continuation, fallthrough)?;
        continuation_block.parameters =
            vec![BackendBlockParameter { value: result, ty: result_type, span }];
        continuation_block.instructions.splice(0..0, merge_instructions);
        append_if_owner_jump(&mut then_block, merge_id, then_value, result_type, span);
        append_if_owner_jump(&mut else_block, merge_id, else_value, result_type, span);
        self.blocks.push(continuation_block);
        self.blocks.push(then_block);
        self.blocks.push(else_block);
        Ok(BackendBlock { parameters: Vec::new(), id, instructions })
    }

    fn lower_if_value_branch(
        &mut self,
        id: BackendBlockId,
        branch: &Block,
        span: SourceSpan,
    ) -> Result<(BackendBlock, BackendValueId), BackendContractError> {
        let Some(Stmt::Expression { expression, .. }) = branch.statements.last() else {
            return Err(unsupported(span, "if value branch requires a tail expression"));
        };
        self.provider.enter_scope(branch.span);
        let mut instructions = Vec::new();
        for statement in &branch.statements[..branch.statements.len() - 1] {
            self.lower_statement(statement, &mut instructions)?;
        }
        let value = self.provider.lower_expression(expression, &mut instructions)?;
        self.provider.exit_scope(branch.span, &mut instructions)?;
        Ok((BackendBlock { parameters: Vec::new(), id, instructions }, value))
    }

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

fn append_if_owner_jump(
    block: &mut BackendBlock,
    merge_id: BackendBlockId,
    value: BackendValueId,
    value_type: crate::codegen::BackendValueType,
    span: SourceSpan,
) {
    block.instructions.push(BackendInstruction::Jump {
        target: merge_id,
        arguments: vec![BackendEdgeArgument { value, ty: value_type, span }],
        span,
    });
}
