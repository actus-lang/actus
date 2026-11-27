use super::contract::unsupported;
use super::{
    BackendBlock, BackendBlockId, BackendBlockParameter, BackendContractError, BackendEdgeArgument,
    BackendExpressionProvider, BackendInstruction, BodyLowering,
};
use crate::ast::{Block, CaseMode, Expr, PatternBinding, Place, Role, Stmt, VariantPayload};
use crate::codegen::BackendValueType;

#[derive(Clone, Copy)]
enum TryTarget<'a> {
    Owner { name: &'a str, declared_type: Option<&'a str> },
}

struct TryOwnerContext<'a> {
    id: BackendBlockId,
    block: &'a Block,
    index: usize,
    target: TryTarget<'a>,
    fallthrough: Option<BackendBlockId>,
    instructions: Vec<BackendInstruction>,
    span: crate::lexer::SourceSpan,
    source: super::BackendValueId,
    condition: super::BackendValueId,
    ok_block: BackendBlockId,
    error_block: BackendBlockId,
    merge_block: BackendBlockId,
    ok_instructions: Vec<BackendInstruction>,
    payload_value: super::BackendValueId,
    payload_type: BackendValueType,
}

struct TryExpressionContext<'a> {
    id: BackendBlockId,
    block: &'a Block,
    index: usize,
    expression: &'a Expr,
    span: crate::lexer::SourceSpan,
    fallthrough: Option<BackendBlockId>,
    instructions: Vec<BackendInstruction>,
}

/// The instructions of a path that leaves the verb with `value` at a `?`:
/// the cleanup the analyzer planned for the exit at `exit` (the `?` itself,
/// or the `return` it is part of), the scalar loan write-backs, and the
/// return (ADR-0125).
fn exit_return<P: BackendExpressionProvider>(
    provider: &mut P,
    value: super::BackendValueId,
    exit: crate::lexer::SourceSpan,
    span: crate::lexer::SourceSpan,
) -> Result<Vec<BackendInstruction>, BackendContractError> {
    let mut instructions = Vec::new();
    provider.emit_return_cleanup(exit, &mut instructions)?;
    provider.append_parameter_writebacks(&mut instructions);
    instructions.push(BackendInstruction::Return { value: Some(value), span });
    Ok(instructions)
}

struct TryTargetContext<'a> {
    id: BackendBlockId,
    block: &'a Block,
    index: usize,
    expression: &'a Expr,
    span: crate::lexer::SourceSpan,
    fallthrough: Option<BackendBlockId>,
    instructions: Vec<BackendInstruction>,
    target: TryTarget<'a>,
}

struct TryBranchesContext {
    ok_block: BackendBlockId,
    error_block: BackendBlockId,
    merge_block: BackendBlockId,
    source: super::BackendValueId,
    span: crate::lexer::SourceSpan,
    payload_value: super::BackendValueId,
    payload_type: BackendValueType,
    ok_instructions: Vec<BackendInstruction>,
}

impl<P: BackendExpressionProvider> BodyLowering<'_, P> {
    pub(super) fn lower_try_statement(
        &mut self,
        id: BackendBlockId,
        block: &Block,
        index: usize,
        statement: &Stmt,
        fallthrough: Option<BackendBlockId>,
        instructions: Vec<BackendInstruction>,
    ) -> Result<BackendBlock, BackendContractError> {
        match statement {
            Stmt::OwnerDecl { .. } => self.lower_try_owner_statement(
                id,
                block,
                index,
                statement,
                fallthrough,
                instructions,
            ),
            Stmt::Assignment { .. } => self.lower_try_assignment_statement(
                id,
                block,
                index,
                statement,
                fallthrough,
                instructions,
            ),
            Stmt::Return { value: Some(Expr::Try { expression, span }), span: exit } => {
                self.lower_try_return_statement(id, *exit, expression, *span, instructions)
            }
            Stmt::Expression { expression: Expr::Try { expression, span }, .. } => self
                .lower_try_expression_statement(TryExpressionContext {
                    id,
                    block,
                    index,
                    expression,
                    span: *span,
                    fallthrough,
                    instructions,
                }),
            _ => unreachable!(),
        }
    }

    fn lower_try_assignment_statement(
        &mut self,
        id: BackendBlockId,
        block: &Block,
        index: usize,
        statement: &Stmt,
        fallthrough: Option<BackendBlockId>,
        instructions: Vec<BackendInstruction>,
    ) -> Result<BackendBlock, BackendContractError> {
        let Stmt::Assignment { target, value: value @ Expr::Try { .. }, span } = statement else {
            unreachable!();
        };
        let previous = match target {
            Place::Binding { name, .. } => self.provider.binding_value(name),
            _ => None,
        };
        let synthetic = synthetic_try_assignment_block(block, index, target, value, *span);
        let result = self.lower_try_owner_statement(
            id,
            &synthetic,
            index,
            &synthetic.statements[index],
            fallthrough,
            instructions,
        );
        if let (Place::Binding { name, .. }, Some(previous)) = (target, previous) {
            self.provider.set_binding_value(name, previous);
        }
        result
    }

    fn lower_try_expression_statement(
        &mut self,
        context: TryExpressionContext<'_>,
    ) -> Result<BackendBlock, BackendContractError> {
        let TryExpressionContext {
            id,
            block,
            index,
            expression,
            span,
            fallthrough,
            mut instructions,
        } = context;
        let source = self.provider.lower_expression(expression, &mut instructions)?;
        let condition = self.provider.lower_enum_variant_condition_value(
            source,
            "Ok",
            span,
            &mut instructions,
        )?;
        let ok_block = self.allocate_block();
        let error_block = self.allocate_block();
        let continuation = self.allocate_block();
        self.blocks.push(BackendBlock {
            parameters: Vec::new(),
            id: ok_block,
            instructions: vec![BackendInstruction::Jump {
                target: continuation,
                arguments: Vec::new(),
                span,
            }],
        });
        self.blocks.push(BackendBlock {
            parameters: Vec::new(),
            id: error_block,
            instructions: exit_return(self.provider, source, span, span)?,
        });
        let continuation_block =
            Block { statements: block.statements[index + 1..].to_vec(), span: block.span };
        let continuation_block =
            self.lower_block_in_scope(continuation, &continuation_block, fallthrough)?;
        self.blocks.push(continuation_block);
        instructions.push(BackendInstruction::Branch {
            condition,
            then_block: ok_block,
            else_block: error_block,
            then_arguments: Vec::new(),
            else_arguments: Vec::new(),
            span,
        });
        Ok(BackendBlock { parameters: Vec::new(), id, instructions })
    }

    fn lower_try_return_statement(
        &mut self,
        id: BackendBlockId,
        exit: crate::lexer::SourceSpan,
        expression: &Expr,
        span: crate::lexer::SourceSpan,
        mut instructions: Vec<BackendInstruction>,
    ) -> Result<BackendBlock, BackendContractError> {
        let source = self.provider.lower_expression(expression, &mut instructions)?;
        let BackendValueType::Enum { layout } = self
            .provider
            .value_type(source)
            .ok_or_else(|| unsupported(span, "try return value type is unavailable"))?
        else {
            return Err(unsupported(span, "try return requires a Result value"));
        };
        let condition = self.provider.lower_enum_variant_condition_value(
            source,
            "Ok",
            span,
            &mut instructions,
        )?;
        let ok_block = self.allocate_block();
        let error_block = self.allocate_block();
        let (mut ok_instructions, payload_value, payload_type) =
            self.lower_try_payload(source, span)?;
        let result = self.provider.allocate_value(BackendValueType::Enum { layout });
        ok_instructions.push(BackendInstruction::EnumConstruct {
            result,
            discriminant: 0,
            payload: Some(payload_value),
            layout,
            payload_type: Some(payload_type),
            span,
        });
        ok_instructions.extend(exit_return(self.provider, result, exit, span)?);
        self.blocks.push(BackendBlock {
            parameters: Vec::new(),
            id: ok_block,
            instructions: ok_instructions,
        });
        self.blocks.push(BackendBlock {
            parameters: Vec::new(),
            id: error_block,
            instructions: exit_return(self.provider, source, exit, span)?,
        });
        instructions.push(BackendInstruction::Branch {
            condition,
            then_block: ok_block,
            else_block: error_block,
            then_arguments: Vec::new(),
            else_arguments: Vec::new(),
            span,
        });
        Ok(BackendBlock { parameters: Vec::new(), id, instructions })
    }

    pub(super) fn lower_try_owner_statement(
        &mut self,
        id: BackendBlockId,
        block: &Block,
        index: usize,
        statement: &Stmt,
        fallthrough: Option<BackendBlockId>,
        instructions: Vec<BackendInstruction>,
    ) -> Result<BackendBlock, BackendContractError> {
        let Stmt::OwnerDecl { name, ty, initializer: Expr::Try { expression, span }, .. } =
            statement
        else {
            return Err(unsupported(crate::lexer::SourceSpan::new(0, 0), "try owner expected"));
        };
        self.lower_try_target(TryTargetContext {
            id,
            block,
            index,
            expression,
            span: *span,
            fallthrough,
            instructions,
            target: TryTarget::Owner { name, declared_type: ty.as_deref() },
        })
    }

    fn lower_try_target(
        &mut self,
        context: TryTargetContext<'_>,
    ) -> Result<BackendBlock, BackendContractError> {
        let TryTargetContext {
            id,
            block,
            index,
            expression,
            span,
            fallthrough,
            mut instructions,
            target,
        } = context;
        let source = self.provider.lower_expression(expression, &mut instructions)?;
        let condition = self.provider.lower_enum_variant_condition_value(
            source,
            "Ok",
            span,
            &mut instructions,
        )?;
        let ok_block = self.allocate_block();
        let error_block = self.allocate_block();
        let merge_block = self.allocate_block();
        let (ok_instructions, payload_value, payload_type) =
            self.lower_try_payload(source, span)?;
        self.finish_try_owner(TryOwnerContext {
            id,
            block,
            index,
            target,
            fallthrough,
            instructions,
            span,
            source,
            condition,
            ok_block,
            error_block,
            merge_block,
            ok_instructions,
            payload_value,
            payload_type,
        })
    }

    fn finish_try_owner(
        &mut self,
        mut context: TryOwnerContext<'_>,
    ) -> Result<BackendBlock, BackendContractError> {
        self.push_try_branches(TryBranchesContext {
            ok_block: context.ok_block,
            error_block: context.error_block,
            merge_block: context.merge_block,
            source: context.source,
            span: context.span,
            payload_value: context.payload_value,
            payload_type: context.payload_type,
            ok_instructions: context.ok_instructions,
        })?;
        append_try_condition_branch(
            &mut context.instructions,
            context.condition,
            context.ok_block,
            context.error_block,
            context.span,
        );
        let result = self.provider.allocate_value(context.payload_type);
        let binding_instructions = self.bind_try_owner(context.target, result, context.span)?;
        let continuation = Block {
            statements: context.block.statements[context.index + 1..].to_vec(),
            span: context.block.span,
        };
        let mut continuation_block =
            self.lower_block_in_scope(context.merge_block, &continuation, context.fallthrough)?;
        continuation_block.parameters = vec![BackendBlockParameter {
            value: result,
            ty: context.payload_type,
            span: context.span,
        }];
        continuation_block.instructions.splice(0..0, binding_instructions);
        self.blocks.push(continuation_block);
        Ok(BackendBlock {
            parameters: Vec::new(),
            id: context.id,
            instructions: context.instructions,
        })
    }

    fn bind_try_owner(
        &mut self,
        target: TryTarget<'_>,
        result: super::BackendValueId,
        span: crate::lexer::SourceSpan,
    ) -> Result<Vec<BackendInstruction>, BackendContractError> {
        let mut instructions = Vec::new();
        match target {
            TryTarget::Owner { name, declared_type } => {
                self.provider.bind_owner(name, result, declared_type, span, &mut instructions)?
            }
        }
        Ok(instructions)
    }

    fn push_try_branches(
        &mut self,
        context: TryBranchesContext,
    ) -> Result<(), BackendContractError> {
        let TryBranchesContext {
            ok_block,
            error_block,
            merge_block,
            source,
            span,
            payload_value,
            payload_type,
            mut ok_instructions,
        } = context;
        ok_instructions.push(BackendInstruction::Jump {
            target: merge_block,
            arguments: vec![BackendEdgeArgument { value: payload_value, ty: payload_type, span }],
            span,
        });
        self.blocks.push(BackendBlock {
            parameters: Vec::new(),
            id: ok_block,
            instructions: ok_instructions,
        });
        self.blocks.push(BackendBlock {
            parameters: Vec::new(),
            id: error_block,
            instructions: exit_return(self.provider, source, span, span)?,
        });
        Ok(())
    }

    fn lower_try_payload(
        &mut self,
        source: super::BackendValueId,
        span: crate::lexer::SourceSpan,
    ) -> Result<
        (Vec<BackendInstruction>, super::BackendValueId, BackendValueType),
        BackendContractError,
    > {
        let payload_name = format!("__try_payload_{}_{}", span.start, span.end);
        let payload = PatternBinding { name: payload_name.clone(), span };
        let mut instructions = Vec::new();
        self.provider.enter_scope(span);
        self.provider.bind_enum_variant_payload(
            source,
            "Ok",
            Some(&VariantPayload::Positional(vec![payload])),
            CaseMode::Dat,
            span,
            &mut instructions,
        )?;
        let value = self
            .provider
            .binding_value(&payload_name)
            .ok_or_else(|| unsupported(span, "try Ok payload binding is unavailable"))?;
        let ty = self
            .provider
            .value_type(value)
            .ok_or_else(|| unsupported(span, "try Ok payload type is unavailable"))?;
        self.provider.exit_scope_without_cleanup()?;
        Ok((instructions, value, ty))
    }
}

fn append_try_condition_branch(
    instructions: &mut Vec<BackendInstruction>,
    condition: super::BackendValueId,
    ok_block: BackendBlockId,
    error_block: BackendBlockId,
    span: crate::lexer::SourceSpan,
) {
    instructions.push(BackendInstruction::Branch {
        condition,
        then_block: ok_block,
        else_block: error_block,
        then_arguments: Vec::new(),
        else_arguments: Vec::new(),
        span,
    });
}

fn synthetic_try_assignment_block(
    block: &Block,
    index: usize,
    target: &Place,
    value: &Expr,
    span: crate::lexer::SourceSpan,
) -> Block {
    let name = format!("__try_assignment_{}_{}", span.start, span.end);
    let mut statements = block.statements.clone();
    statements[index] = Stmt::OwnerDecl {
        role: Role::Erg,
        name: name.clone(),
        ty: None,
        resolved_type: None,
        initializer: value.clone(),
        span,
    };
    statements.insert(
        index + 1,
        Stmt::Assignment { target: target.clone(), value: Expr::Identifier { name, span }, span },
    );
    Block { statements, span: block.span }
}
