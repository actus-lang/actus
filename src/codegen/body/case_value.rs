use super::super::BackendBlockParameter;
use super::super::body_contract::unsupported;
use super::{
    BackendBlock, BackendBlockId, BackendContractError, BackendEdgeArgument,
    BackendExpressionProvider, BackendInstruction, BackendValueId, BodyLowering,
};
use crate::ast::{Block, CaseBody, Expr, Pattern, Stmt, VariantPayload};
use crate::codegen::BackendValueType;
use crate::lexer::SourceSpan;

struct CaseOwnerBranches {
    merge_id: BackendBlockId,
    branch_values: Vec<(BackendBlockId, BackendValueId)>,
    current_id: BackendBlockId,
    entry_instructions: Option<Vec<BackendInstruction>>,
    instructions: Vec<BackendInstruction>,
}

pub(super) struct CaseOwnerInput<'a> {
    pub(super) id: BackendBlockId,
    pub(super) block: &'a Block,
    pub(super) index: usize,
    pub(super) name: &'a str,
    pub(super) declared_type: Option<&'a str>,
    pub(super) expression: &'a Expr,
    pub(super) span: SourceSpan,
    pub(super) fallthrough: Option<BackendBlockId>,
    pub(super) instructions: Vec<BackendInstruction>,
}

impl<P: BackendExpressionProvider> BodyLowering<'_, P> {
    pub(super) fn lower_case_owner(
        &mut self,
        input: CaseOwnerInput<'_>,
    ) -> Result<BackendBlock, BackendContractError> {
        let CaseOwnerInput {
            id,
            block,
            index,
            name,
            declared_type,
            expression,
            span,
            fallthrough,
            instructions,
        } = input;
        let Expr::Case { mode, subject, branches, .. } = expression else {
            return Err(unsupported(span, "enum case value expected"));
        };
        let CaseOwnerBranches {
            merge_id,
            branch_values,
            current_id,
            entry_instructions,
            mut instructions,
        } = self.lower_case_owner_branches(id, *mode, subject, branches, span, instructions)?;
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
        self.attach_case_value_branches(branch_values, merge_id, result_type, span)?;
        if let Some(entry_instructions) = entry_instructions {
            self.blocks.push(BackendBlock { parameters: Vec::new(), id: current_id, instructions });
            instructions = entry_instructions;
        }
        Ok(BackendBlock { parameters: Vec::new(), id, instructions })
    }

    fn attach_case_value_branches(
        &mut self,
        branch_values: Vec<(BackendBlockId, BackendValueId)>,
        merge_id: BackendBlockId,
        result_type: BackendValueType,
        span: SourceSpan,
    ) -> Result<(), BackendContractError> {
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
        Ok(())
    }

    fn lower_case_owner_branches(
        &mut self,
        id: BackendBlockId,
        mode: crate::ast::CaseMode,
        subject: &Expr,
        branches: &[crate::ast::CaseBranch],
        span: SourceSpan,
        mut instructions: Vec<BackendInstruction>,
    ) -> Result<CaseOwnerBranches, BackendContractError> {
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
            let (matched, branch_value) =
                self.lower_case_value_branch(matched_id, branch, mode, subject_value, span)?;
            if let Some((value_block, value)) = branch_value {
                branch_values.push((value_block, value));
            }
            self.blocks.push(matched);
            self.advance_case_owner_branch(
                following_id,
                &mut current_id,
                &mut entry_instructions,
                &mut instructions,
                else_id,
                span,
            );
        }
        Ok(CaseOwnerBranches {
            merge_id,
            branch_values,
            current_id,
            entry_instructions,
            instructions,
        })
    }

    fn advance_case_owner_branch(
        &mut self,
        following_id: Option<BackendBlockId>,
        current_id: &mut BackendBlockId,
        entry_instructions: &mut Option<Vec<BackendInstruction>>,
        instructions: &mut Vec<BackendInstruction>,
        else_id: BackendBlockId,
        span: SourceSpan,
    ) {
        if let Some(following_id) = following_id {
            if entry_instructions.is_none() {
                *entry_instructions = Some(std::mem::take(instructions));
            } else {
                self.blocks.push(BackendBlock {
                    parameters: Vec::new(),
                    id: *current_id,
                    instructions: std::mem::take(instructions),
                });
            }
            *current_id = following_id;
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

    fn lower_case_value_branch(
        &mut self,
        id: BackendBlockId,
        branch: &crate::ast::CaseBranch,
        mode: crate::ast::CaseMode,
        subject: super::BackendValueId,
        span: SourceSpan,
    ) -> Result<(BackendBlock, Option<(BackendBlockId, super::BackendValueId)>), BackendContractError>
    {
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
            CaseBody::Expression(expression)
                if matches!(expression.as_ref(), Expr::Case { .. }) =>
            {
                let (entry, merge_id, value) =
                    self.lower_nested_case_value(id, expression, span, instructions)?;
                return Ok((entry, Some((merge_id, value))));
            }
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
        Ok((BackendBlock { parameters: Vec::new(), id, instructions }, Some((id, value))))
    }

    fn lower_nested_case_value(
        &mut self,
        id: BackendBlockId,
        expression: &Expr,
        span: SourceSpan,
        instructions: Vec<BackendInstruction>,
    ) -> Result<(BackendBlock, BackendBlockId, BackendValueId), BackendContractError> {
        let Expr::Case { mode, subject, branches, .. } = expression else {
            return Err(unsupported(span, "nested enum case expected"));
        };
        let lowered =
            self.lower_case_owner_branches(id, *mode, subject, branches, span, instructions)?;
        let Some((_, first_value)) = lowered.branch_values.first().copied() else {
            return Err(unsupported(span, "empty nested enum case"));
        };
        let result_type = self
            .provider
            .value_type(first_value)
            .ok_or_else(|| unsupported(span, "nested case result type is unavailable"))?;
        let result = self.provider.allocate_value(result_type);
        self.blocks.push(BackendBlock {
            parameters: vec![BackendBlockParameter { value: result, ty: result_type, span }],
            id: lowered.merge_id,
            instructions: Vec::new(),
        });
        self.attach_case_value_branches(
            lowered.branch_values,
            lowered.merge_id,
            result_type,
            span,
        )?;
        let entry = if let Some(entry_instructions) = lowered.entry_instructions {
            self.blocks.push(BackendBlock {
                parameters: Vec::new(),
                id: lowered.current_id,
                instructions: lowered.instructions,
            });
            BackendBlock { parameters: Vec::new(), id, instructions: entry_instructions }
        } else {
            BackendBlock { parameters: Vec::new(), id, instructions: lowered.instructions }
        };
        Ok((entry, lowered.merge_id, result))
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
