use super::super::BackendBlockParameter;
use super::super::body_contract::unsupported;
use super::{
    BackendBlock, BackendBlockId, BackendContractError, BackendEdgeArgument,
    BackendExpressionProvider, BackendInstruction, BodyLowering,
};
use crate::ast::{Block, CaseBody, Expr, Pattern, Place, Stmt, VariantPayload};
use crate::lexer::SourceSpan;

impl<P: BackendExpressionProvider> BodyLowering<'_, P> {
    pub(super) fn lower_case_assignment(
        &mut self,
        id: BackendBlockId,
        block: &Block,
        index: usize,
        target: &Place,
        expression: &Expr,
        span: SourceSpan,
        fallthrough: Option<BackendBlockId>,
        instructions: Vec<BackendInstruction>,
    ) -> Result<BackendBlock, BackendContractError> {
        let name = format!("__case_value_{}", id.0);
        let mut statements = block.statements.clone();
        statements.insert(
            index + 1,
            Stmt::Assignment {
                target: target.clone(),
                value: Expr::Identifier { name: name.clone(), span },
                span,
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
            span,
            fallthrough,
            instructions,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn lower_case_owner(
        &mut self,
        id: BackendBlockId,
        block: &Block,
        index: usize,
        name: &str,
        declared_type: Option<&str>,
        expression: &Expr,
        span: SourceSpan,
        fallthrough: Option<BackendBlockId>,
        mut instructions: Vec<BackendInstruction>,
    ) -> Result<BackendBlock, BackendContractError> {
        let Expr::Case { mode, subject, branches, .. } = expression else {
            return Err(unsupported(span, "enum case value expected"));
        };
        let merge_id = self.allocate_block();
        let mut branch_values = Vec::new();
        let mut current_id = id;
        let mut entry_instructions = None;
        for (index, branch) in branches.iter().enumerate() {
            let matched_id = self.allocate_block();
            let is_last = index + 1 == branches.len();
            let following_id = (!is_last).then(|| self.allocate_block());
            let (subject_value, condition) = super::case::lower_case_condition(
                self,
                subject,
                branch,
                is_last,
                &mut instructions,
            )?;
            let else_id = following_id.unwrap_or_else(|| self.allocate_block());
            instructions.push(BackendInstruction::Branch {
                condition,
                then_block: matched_id,
                else_block: else_id,
                then_arguments: Vec::new(),
                else_arguments: Vec::new(),
                span: branch.span,
            });
            let (matched, value) =
                self.lower_case_value_branch(matched_id, branch, *mode, subject_value, span)?;
            if let Some(value) = value {
                branch_values.push((matched.id, value));
            }
            self.blocks.push(matched);
            if let Some(following_id) = following_id {
                if entry_instructions.is_none() {
                    entry_instructions = Some(std::mem::take(&mut instructions));
                } else {
                    self.blocks.push(BackendBlock {
                        parameters: Vec::new(),
                        id: current_id,
                        instructions,
                    });
                    instructions = Vec::new();
                }
                current_id = following_id;
            } else {
                self.blocks.push(BackendBlock {
                    parameters: Vec::new(),
                    id: else_id,
                    instructions: vec![BackendInstruction::Trap {
                        reason: "non-exhaustive enum case".to_owned(),
                        span,
                    }],
                });
            }
        }
        let Some((_, first_value)) = branch_values.first().copied() else {
            return Err(unsupported(span, "empty enum case"));
        };
        let result_type = self
            .provider
            .value_type(first_value)
            .ok_or_else(|| unsupported(span, "case result type is unavailable"))?;
        let result = self.provider.allocate_value(result_type);
        let mut merge_instructions = Vec::new();
        self.provider.bind_owner(name, result, declared_type, span, &mut merge_instructions)?;
        let continuation =
            Block { statements: block.statements[index + 1..].to_vec(), span: block.span };
        let mut continuation_block =
            self.lower_block_in_scope(merge_id, &continuation, fallthrough)?;
        continuation_block.parameters =
            vec![BackendBlockParameter { value: result, ty: result_type, span }];
        self.blocks.push(continuation_block);
        for (branch_id, value) in branch_values {
            let value_type = self
                .provider
                .value_type(value)
                .ok_or_else(|| unsupported(span, "case branch type is unavailable"))?;
            if value_type != result_type {
                return Err(unsupported(span, "case branches have different value types"));
            }
            if let Some(branch) = self.blocks.iter_mut().find(|candidate| candidate.id == branch_id)
            {
                branch.instructions.push(BackendInstruction::Jump {
                    target: merge_id,
                    arguments: vec![BackendEdgeArgument { value, ty: result_type, span }],
                    span,
                });
            }
        }
        if let Some(entry_instructions) = entry_instructions {
            self.blocks.push(BackendBlock { parameters: Vec::new(), id: current_id, instructions });
            instructions = entry_instructions;
        }
        Ok(BackendBlock { parameters: Vec::new(), id, instructions })
    }

    fn lower_case_value_branch(
        &mut self,
        id: BackendBlockId,
        branch: &crate::ast::CaseBranch,
        mode: crate::ast::CaseMode,
        subject: super::BackendValueId,
        span: SourceSpan,
    ) -> Result<(BackendBlock, Option<super::BackendValueId>), BackendContractError> {
        self.provider.enter_scope(branch.span);
        let mut instructions = Vec::new();
        if matches!(branch.pattern, Pattern::Variant { .. }) {
            self.provider.bind_enum_variant_payload(
                subject,
                branch_variant(branch),
                branch_payload(branch),
                mode,
                branch.span,
                &mut instructions,
            )?;
        }
        let value = match &branch.body {
            CaseBody::Expression(expression) => {
                self.provider.lower_expression(expression, &mut instructions)?
            }
            CaseBody::Block(block) => {
                let Some(last) = block.statements.last() else {
                    return Err(unsupported(span, "case value block is empty"));
                };
                if matches!(last, Stmt::Return { .. }) {
                    for statement in &block.statements {
                        self.lower_statement(statement, &mut instructions)?;
                    }
                    self.provider.exit_scope(branch.span, &mut instructions)?;
                    return Ok((BackendBlock { parameters: Vec::new(), id, instructions }, None));
                }
                let Stmt::Expression { expression, .. } = last else {
                    return Err(unsupported(span, "case value block requires a tail expression"));
                };
                for statement in &block.statements[..block.statements.len() - 1] {
                    self.lower_statement(statement, &mut instructions)?;
                }
                self.provider.lower_expression(expression, &mut instructions)?
            }
        };
        self.provider.exit_scope(branch.span, &mut instructions)?;
        self.provider.value_type(value).ok_or_else(|| unsupported(span, "case branch type"))?;
        Ok((BackendBlock { parameters: Vec::new(), id, instructions }, Some(value)))
    }
}

fn branch_variant(branch: &crate::ast::CaseBranch) -> &str {
    match &branch.pattern {
        Pattern::Variant { variant, .. } => variant,
        _ => "_",
    }
}

fn branch_payload(branch: &crate::ast::CaseBranch) -> Option<&VariantPayload> {
    match &branch.pattern {
        Pattern::Variant { payload, .. } => Some(payload),
        _ => None,
    }
}
