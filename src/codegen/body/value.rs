use super::contract::unsupported;
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
    /// Each branch block with the value it produces, in source order.
    arms: Vec<(BackendBlock, BackendValueId)>,
    /// The condition blocks of `else if` arms, which produce no value.
    tests: Vec<BackendBlock>,
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
        append_if_owner_branch(&mut instructions, condition, then_id, else_id, *span);
        let (then_block, then_value) = self.lower_if_value_branch(then_id, then_branch, *span)?;
        let mut arms = vec![(then_block, then_value)];
        let mut tests = Vec::new();
        let returning_else =
            self.lower_owner_else(else_id, else_branch.as_ref(), *span, &mut arms, &mut tests)?;
        let input = IfOwnerInput {
            id,
            block,
            index,
            name,
            declared_type: ty.as_deref(),
            span: *span,
            fallthrough,
            merge_id,
            instructions,
            arms,
            tests,
        };
        match returning_else {
            Some(branch) => self.finish_if_owner_return(input, else_id, branch),
            None => self.finish_if_owner(input),
        }
    }

    /// Lowers the `else` of an owner `if` into `arms` and `tests`, or returns
    /// the `else` block when it ends with `return` and so produces no value.
    fn lower_owner_else<'a>(
        &mut self,
        else_id: BackendBlockId,
        else_branch: Option<&'a IfBranch>,
        span: SourceSpan,
        arms: &mut Vec<(BackendBlock, BackendValueId)>,
        tests: &mut Vec<BackendBlock>,
    ) -> Result<Option<&'a Block>, BackendContractError> {
        match else_branch {
            Some(IfBranch::Block(branch))
                if matches!(branch.statements.last(), Some(Stmt::Return { .. })) =>
            {
                Ok(Some(branch))
            }
            Some(IfBranch::Block(branch)) => {
                arms.push(self.lower_if_value_branch(else_id, branch, span)?);
                Ok(None)
            }
            Some(IfBranch::ElseIf(nested)) => {
                self.lower_else_if_arms(else_id, nested, arms, tests)?;
                Ok(None)
            }
            None => Err(unsupported(span, "value-producing if requires an else branch")),
        }
    }

    /// Finishes an owner `if` whose only value arm is `then`; its `else`
    /// branch, `else_branch` at block `else_id`, returns from the verb.
    fn finish_if_owner_return(
        &mut self,
        input: IfOwnerInput<'_>,
        else_id: BackendBlockId,
        else_branch: &Block,
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
            arms,
            ..
        } = input;
        let Some((mut then_block, then_value)) = arms.into_iter().next() else {
            return Err(unsupported(span, "if owner has no value arm"));
        };
        let else_block = self.return_branch(else_id, else_branch, span)?;
        let result_type = self
            .provider
            .value_type(then_value)
            .ok_or_else(|| unsupported(span, "if branch result type is unavailable"))?;
        let result = self.provider.allocate_value(result_type);
        self.provider.copy_value_metadata(then_value, result);
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
        self.blocks.push(continuation_block);
        self.blocks.push(then_block);
        self.blocks.push(else_block);
        Ok(BackendBlock { parameters: Vec::new(), id, instructions })
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
            arms,
            tests,
        } = input;
        let first_value = arms[0].1;
        let result_type = self
            .provider
            .value_type(first_value)
            .ok_or_else(|| unsupported(span, "if branch result type is unavailable"))?;
        if arms.iter().any(|(_, value)| self.provider.value_type(*value) != Some(result_type)) {
            return Err(unsupported(span, "if branches have different value types"));
        }
        let result = self.provider.allocate_value(result_type);
        self.provider.copy_value_metadata(first_value, result);
        let mut merge_instructions = Vec::new();
        self.provider.bind_owner(name, result, declared_type, span, &mut merge_instructions)?;
        let continuation =
            Block { statements: block.statements[index + 1..].to_vec(), span: block.span };
        let mut continuation_block =
            self.lower_block_in_scope(merge_id, &continuation, fallthrough)?;
        continuation_block.parameters =
            vec![BackendBlockParameter { value: result, ty: result_type, span }];
        continuation_block.instructions.splice(0..0, merge_instructions);
        self.blocks.push(continuation_block);
        self.blocks.extend(tests);
        for (mut arm, value) in arms {
            append_if_owner_jump(&mut arm, merge_id, value, result_type, span);
            self.blocks.push(arm);
        }
        Ok(BackendBlock { parameters: Vec::new(), id, instructions })
    }

    /// Lowers the `else if` chain `nested` starting at block `test_id`: each
    /// condition gets a test block that branches to its arm or to the next
    /// test, so only the selected arm runs.
    fn lower_else_if_arms(
        &mut self,
        test_id: BackendBlockId,
        nested: &Expr,
        arms: &mut Vec<(BackendBlock, BackendValueId)>,
        tests: &mut Vec<BackendBlock>,
    ) -> Result<(), BackendContractError> {
        let Expr::If { condition, then_branch, else_branch, span } = nested else {
            return Err(unsupported(crate::parser::expression_span(nested), "`else if` expected"));
        };
        let mut instructions = Vec::new();
        let condition = self.provider.lower_expression(condition, &mut instructions)?;
        let then_id = self.allocate_block();
        let next_id = self.allocate_block();
        append_if_owner_branch(&mut instructions, condition, then_id, next_id, *span);
        tests.push(BackendBlock { parameters: Vec::new(), id: test_id, instructions });
        arms.push(self.lower_if_value_branch(then_id, then_branch, *span)?);
        match else_branch {
            Some(IfBranch::Block(block)) => {
                arms.push(self.lower_if_value_branch(next_id, block, *span)?);
                Ok(())
            }
            Some(IfBranch::ElseIf(next)) => self.lower_else_if_arms(next_id, next, arms, tests),
            None => Err(unsupported(*span, "value-producing if requires an else branch")),
        }
    }

    fn lower_if_value_branch(
        &mut self,
        id: BackendBlockId,
        branch: &Block,
        span: SourceSpan,
    ) -> Result<(BackendBlock, BackendValueId), BackendContractError> {
        if let Some(Stmt::If { condition, then_branch, else_branch, .. }) = branch.statements.last()
            && let Expr::BoolLiteral { value, .. } = condition
        {
            let selected = if *value {
                then_branch
            } else if let Some(IfBranch::Block(block)) = else_branch {
                block
            } else {
                return Err(unsupported(span, "constant if value branch has no selected block"));
            };
            return self.lower_if_value_branch(id, selected, span);
        }
        let Some(Stmt::Expression { expression, .. }) = branch.statements.last() else {
            return Err(unsupported(span, "if value branch requires a tail expression"));
        };
        self.provider.enter_scope(branch.span);
        let mut instructions = Vec::new();
        for statement in &branch.statements[..branch.statements.len() - 1] {
            self.lower_statement(statement, &mut instructions)?;
        }
        let value = self.provider.lower_expression(expression, &mut instructions)?;
        // The branch hands its value to the merge block here, which ends its scope.
        self.provider.emit_scope_end(branch.span, &mut instructions);
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
        // The branch leaves the verb with a `return` of its own, whose exit
        // list the analyzer planned under the branch's span (ADR-0125): its
        // value is returned, or, in a verb without a result, the branch runs
        // as statements and then returns. A branch the lowering synthesised
        // has no list and leaves through the `return` statement at `span`.
        let exit_span = if self.provider.has_return_exit(branch.span) { branch.span } else { span };
        let exit = Stmt::Return { value: None, span: exit_span };
        match last {
            Stmt::Expression { expression, .. } if !self.returns_unit() => {
                *last = Stmt::Return { value: Some(expression.clone()), span: exit_span };
            }
            Stmt::Return { .. } | Stmt::Break { .. } | Stmt::Continue { .. } => {}
            _ if self.returns_unit() => branch.statements.push(exit),
            _ => {}
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

fn append_if_owner_branch(
    instructions: &mut Vec<BackendInstruction>,
    condition: BackendValueId,
    then_block: BackendBlockId,
    else_block: BackendBlockId,
    span: SourceSpan,
) {
    instructions.push(BackendInstruction::Branch {
        condition,
        then_block,
        else_block,
        then_arguments: Vec::new(),
        else_arguments: Vec::new(),
        span,
    });
}
