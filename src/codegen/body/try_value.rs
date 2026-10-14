use super::super::body_contract::unsupported;
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
            Stmt::Assignment { target, value, span } => {
                let Expr::Try { .. } = value else { unreachable!() };
                let previous = match target {
                    Place::Binding { name, .. } => self.provider.binding_value(name),
                    _ => None,
                };
                let name = format!("__try_assignment_{}_{}", span.start, span.end);
                let mut statements = block.statements.clone();
                statements[index] = Stmt::OwnerDecl {
                    role: Role::Erg,
                    name: name.clone(),
                    ty: None,
                    initializer: value.clone(),
                    span: *span,
                };
                statements.insert(
                    index + 1,
                    Stmt::Assignment {
                        target: target.clone(),
                        value: Expr::Identifier { name, span: *span },
                        span: *span,
                    },
                );
                let synthetic = Block { statements, span: block.span };
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
            Stmt::Return { value: Some(Expr::Try { expression, span }), .. } => {
                self.lower_try_return_statement(id, expression, *span, instructions)
            }
            Stmt::Expression { expression: Expr::Try { expression, span }, .. } => self
                .lower_try_expression_statement(
                    id,
                    block,
                    index,
                    expression,
                    *span,
                    fallthrough,
                    instructions,
                ),
            _ => unreachable!(),
        }
    }

    fn lower_try_expression_statement(
        &mut self,
        id: BackendBlockId,
        block: &Block,
        index: usize,
        expression: &Expr,
        span: crate::lexer::SourceSpan,
        fallthrough: Option<BackendBlockId>,
        mut instructions: Vec<BackendInstruction>,
    ) -> Result<BackendBlock, BackendContractError> {
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
            instructions: vec![BackendInstruction::Return { value: Some(source), span }],
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
        ok_instructions.push(BackendInstruction::Return { value: Some(result), span });
        self.blocks.push(BackendBlock {
            parameters: Vec::new(),
            id: ok_block,
            instructions: ok_instructions,
        });
        self.blocks.push(BackendBlock {
            parameters: Vec::new(),
            id: error_block,
            instructions: vec![BackendInstruction::Return { value: Some(source), span }],
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
        self.lower_try_target(
            id,
            block,
            index,
            expression,
            *span,
            fallthrough,
            instructions,
            TryTarget::Owner { name, declared_type: ty.as_deref() },
        )
    }

    fn lower_try_target(
        &mut self,
        id: BackendBlockId,
        block: &Block,
        index: usize,
        expression: &Expr,
        span: crate::lexer::SourceSpan,
        fallthrough: Option<BackendBlockId>,
        mut instructions: Vec<BackendInstruction>,
        target: TryTarget<'_>,
    ) -> Result<BackendBlock, BackendContractError> {
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
        let (mut ok_instructions, payload_value, payload_type) =
            self.lower_try_payload(source, span)?;
        self.finish_try_owner(
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
            &mut ok_instructions,
            payload_value,
            payload_type,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn finish_try_owner(
        &mut self,
        id: BackendBlockId,
        block: &Block,
        index: usize,
        target: TryTarget<'_>,
        fallthrough: Option<BackendBlockId>,
        mut instructions: Vec<BackendInstruction>,
        span: crate::lexer::SourceSpan,
        source: super::BackendValueId,
        condition: super::BackendValueId,
        ok_block: BackendBlockId,
        error_block: BackendBlockId,
        merge_block: BackendBlockId,
        ok_instructions: &mut Vec<BackendInstruction>,
        payload_value: super::BackendValueId,
        payload_type: BackendValueType,
    ) -> Result<BackendBlock, BackendContractError> {
        self.push_try_branches(
            ok_block,
            error_block,
            merge_block,
            source,
            span,
            payload_value,
            payload_type,
            ok_instructions,
        );
        instructions.push(BackendInstruction::Branch {
            condition,
            then_block: ok_block,
            else_block: error_block,
            then_arguments: Vec::new(),
            else_arguments: Vec::new(),
            span,
        });
        let result = self.provider.allocate_value(payload_type);
        let mut binding_instructions = Vec::new();
        match target {
            TryTarget::Owner { name, declared_type } => {
                self.provider.bind_owner(
                    name,
                    result,
                    declared_type,
                    span,
                    &mut binding_instructions,
                )?;
            }
        }
        let continuation =
            Block { statements: block.statements[index + 1..].to_vec(), span: block.span };
        let mut continuation_block =
            self.lower_block_in_scope(merge_block, &continuation, fallthrough)?;
        continuation_block.parameters =
            vec![BackendBlockParameter { value: result, ty: payload_type, span }];
        continuation_block.instructions.splice(0..0, binding_instructions);
        self.blocks.push(continuation_block);
        Ok(BackendBlock { parameters: Vec::new(), id, instructions })
    }

    fn push_try_branches(
        &mut self,
        ok_block: BackendBlockId,
        error_block: BackendBlockId,
        merge_block: BackendBlockId,
        source: super::BackendValueId,
        span: crate::lexer::SourceSpan,
        payload_value: super::BackendValueId,
        payload_type: BackendValueType,
        ok_instructions: &mut Vec<BackendInstruction>,
    ) {
        ok_instructions.push(BackendInstruction::Jump {
            target: merge_block,
            arguments: vec![BackendEdgeArgument { value: payload_value, ty: payload_type, span }],
            span,
        });
        self.blocks.push(BackendBlock {
            parameters: Vec::new(),
            id: ok_block,
            instructions: std::mem::take(ok_instructions),
        });
        self.blocks.push(BackendBlock {
            parameters: Vec::new(),
            id: error_block,
            instructions: vec![BackendInstruction::Return { value: Some(source), span }],
        });
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
