// Structured control-flow lowering for the body backend.

use super::case_value::CaseOwnerInput;
use super::loop_support::collect_assignment_names;
use super::{BackendBlock, BackendBlockId, BackendContractError, BackendInstruction, BodyLowering};
use crate::ast::{Block, CaseBody, Expr, Stmt};

struct CaseExpressionStatementInput<'a> {
    id: BackendBlockId,
    block: &'a Block,
    index: usize,
    expression: &'a Expr,
    span: crate::lexer::SourceSpan,
    fallthrough: Option<BackendBlockId>,
    instructions: Vec<BackendInstruction>,
}

impl<P: super::BackendExpressionProvider> BodyLowering<'_, P> {
    pub(super) fn lower_case_condition_statement(
        &mut self,
        id: BackendBlockId,
        block: &Block,
        index: usize,
        statement: &Stmt,
        fallthrough: Option<BackendBlockId>,
    ) -> Result<BackendBlock, BackendContractError> {
        let Stmt::If { condition, span, .. } = statement else { unreachable!() };
        let name = format!("__case_condition_{}", id.0);
        let mut statements = block.statements.clone();
        statements[index] = Stmt::OwnerDecl {
            role: crate::ast::Role::Erg,
            name: name.clone(),
            ty: None,
            initializer: condition.clone(),
            span: *span,
        };
        if let Stmt::If { condition, .. } = &mut statements[index + 1] {
            *condition = Expr::Identifier { name, span: *span };
        }
        self.lower_block_in_scope(id, &Block { statements, span: block.span }, fallthrough)
    }

    pub(super) fn lower_structured_statement(
        &mut self,
        id: BackendBlockId,
        block: &Block,
        index: usize,
        statement: &Stmt,
        fallthrough: Option<BackendBlockId>,
        instructions: Vec<BackendInstruction>,
    ) -> Result<Option<BackendBlock>, BackendContractError> {
        if let Stmt::Return { value: Some(expression @ Expr::Case { .. }), span } = statement {
            return self
                .lower_case_return_case(CaseExpressionStatementInput {
                    id,
                    block,
                    index,
                    expression,
                    span: *span,
                    fallthrough,
                    instructions,
                })
                .map(Some);
        }
        self.lower_nonreturn_structured_statement(
            id,
            block,
            index,
            statement,
            fallthrough,
            instructions,
        )
    }

    fn lower_nonreturn_structured_statement(
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
            Stmt::OwnerDecl { initializer: Expr::If { .. }, .. } => self
                .lower_if_owner_statement(id, block, index, statement, fallthrough, instructions)
                .map(Some),
            Stmt::Return { value: Some(expression @ Expr::If { .. }), span } => {
                self.lower_if_return(id, expression, *span, instructions).map(Some)
            }
            Stmt::Expression { expression: expression @ Expr::Case { .. }, span } => self
                .lower_case_expression_statement(CaseExpressionStatementInput {
                    id,
                    block,
                    index,
                    expression,
                    span: *span,
                    fallthrough,
                    instructions,
                })
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

    fn lower_case_return_case(
        &mut self,
        input: CaseExpressionStatementInput<'_>,
    ) -> Result<BackendBlock, BackendContractError> {
        let CaseExpressionStatementInput {
            id,
            block,
            index,
            expression,
            span,
            fallthrough,
            instructions,
        } = input;
        if case_requires_continuation(expression) {
            return self.lower_case_expression_statement(CaseExpressionStatementInput {
                id,
                block,
                index,
                expression,
                span,
                fallthrough,
                instructions,
            });
        }
        self.lower_case_return(id, expression, span, instructions)
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
        self.lower_case_owner(CaseOwnerInput {
            id,
            block,
            index,
            name,
            declared_type: ty.as_deref(),
            expression,
            span: *span,
            fallthrough,
            instructions,
        })
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
        self.lower_case_owner(CaseOwnerInput {
            id,
            block: &synthetic,
            index,
            name: &name,
            declared_type: None,
            expression,
            span: *span,
            fallthrough,
            instructions,
        })
    }

    fn lower_case_expression_statement(
        &mut self,
        input: CaseExpressionStatementInput<'_>,
    ) -> Result<BackendBlock, BackendContractError> {
        let CaseExpressionStatementInput {
            id,
            block,
            index,
            expression,
            span,
            fallthrough,
            mut instructions,
        } = input;
        let continuation_id = self.allocate_block();
        let carried = self.case_carried_bindings(expression);
        let (case_block, continuation_parameters) = if carried.is_empty() {
            (
                self.lower_case_statement(
                    id,
                    expression,
                    span,
                    continuation_id,
                    &mut instructions,
                )?,
                Vec::new(),
            )
        } else {
            self.lower_case_statement_with_carried(
                id,
                expression,
                span,
                continuation_id,
                &carried,
                instructions,
            )?
        };
        let continuation =
            Block { statements: block.statements[index + 1..].to_vec(), span: block.span };
        let mut continuation_block =
            self.lower_block_in_scope(continuation_id, &continuation, fallthrough)?;
        continuation_block.parameters = continuation_parameters;
        self.blocks.push(continuation_block);
        Ok(BackendBlock { parameters: Vec::new(), id, instructions: case_block.instructions })
    }

    fn case_carried_bindings(&self, expression: &Expr) -> Vec<super::loops::CarriedBinding> {
        let Expr::Case { branches, .. } = expression else { return Vec::new() };
        let mut names = Vec::new();
        for branch in branches {
            if let CaseBody::Block(block) = &branch.body {
                collect_assignment_names(block, &mut names);
            }
        }
        names.sort();
        names.dedup();
        names
            .into_iter()
            .filter_map(|name| {
                let value = self.provider.binding_value(&name)?;
                let ty = self.provider.value_type(value)?;
                Some(super::loops::CarriedBinding {
                    name,
                    value,
                    ty,
                    nominal_type: self.provider.nominal_type(value),
                })
            })
            .collect()
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
        self.blocks.push(nested_block);
        let continuation_start = self.blocks.len();
        let continuation =
            Block { statements: block.statements[index + 1..].to_vec(), span: block.span };
        let continuation_block =
            self.lower_block_in_scope(continuation_id, &continuation, fallthrough)?;
        self.blocks.push(continuation_block);
        self.provider.distribute_scope_cleanup(block.span, &mut self.blocks, continuation_start)?;
        Ok(BackendBlock { parameters: Vec::new(), id, instructions })
    }
}

fn case_requires_continuation(expression: &Expr) -> bool {
    let Expr::Case { branches, .. } = expression else { return false };
    branches.iter().any(|branch| {
        let crate::ast::CaseBody::Block(block) = &branch.body else { return false };
        !block.statements.last().is_some_and(|statement| {
            matches!(
                statement,
                Stmt::Expression { .. }
                    | Stmt::If { .. }
                    | Stmt::Return { .. }
                    | Stmt::Break { .. }
                    | Stmt::Continue { .. }
            )
        })
    })
}
