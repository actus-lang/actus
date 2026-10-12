use super::super::body_contract::unsupported;
use super::{
    BackendBlock, BackendBlockId, BackendContractError, BackendExpressionProvider,
    BackendInstruction, BackendValueId, BodyLowering,
};
use crate::ast::{Block, CaseBody, CaseMode, Expr, LiteralPattern, Pattern, Stmt, VariantPayload};
use crate::codegen::BackendValueType;
use crate::lexer::SourceSpan;
impl<P: BackendExpressionProvider> BodyLowering<'_, P> {
    pub(super) fn lower_case_return(
        &mut self,
        id: BackendBlockId,
        expression: &Expr,
        span: SourceSpan,
        instructions: Vec<BackendInstruction>,
    ) -> Result<BackendBlock, BackendContractError> {
        self.lower_case_control_flow(id, expression, span, None, instructions)
    }

    pub(super) fn lower_case_statement(
        &mut self,
        id: BackendBlockId,
        expression: &Expr,
        span: SourceSpan,
        continuation_id: BackendBlockId,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendBlock, BackendContractError> {
        self.lower_case_control_flow(
            id,
            expression,
            span,
            Some(continuation_id),
            std::mem::take(instructions),
        )
    }

    fn lower_case_control_flow(
        &mut self,
        id: BackendBlockId,
        expression: &Expr,
        span: SourceSpan,
        fallthrough: Option<BackendBlockId>,
        mut instructions: Vec<BackendInstruction>,
    ) -> Result<BackendBlock, BackendContractError> {
        let Expr::Case { mode, subject, branches, .. } = expression else {
            return Err(unsupported(span, "enum case expression expected"));
        };
        let mut current_id = id;
        let mut entry_instructions = None;
        for (index, branch) in branches.iter().enumerate() {
            let (is_last, (matched_id, following_id, else_block)) =
                self.prepare_case_branch(index, branches.len(), branch)?;
            let (subject_value, matched) =
                lower_case_condition(self, subject, branch, is_last, &mut instructions)?;
            instructions.push(BackendInstruction::Branch {
                condition: matched,
                then_block: matched_id,
                else_block,
                then_arguments: Vec::new(),
                else_arguments: Vec::new(),
                span: branch.span,
            });
            self.insert_case_branch_body(
                matched_id,
                branch,
                span,
                subject_value,
                *mode,
                fallthrough,
            )?;
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
                return self.finish_case_branch(
                    id,
                    current_id,
                    else_block,
                    span,
                    instructions,
                    entry_instructions,
                );
            }
        }
        Err(unsupported(span, "empty enum case"))
    }

    fn prepare_case_branch(
        &mut self,
        index: usize,
        branch_count: usize,
        branch: &crate::ast::CaseBranch,
    ) -> Result<
        (bool, (BackendBlockId, Option<BackendBlockId>, BackendBlockId)),
        BackendContractError,
    > {
        ensure_case_branch_supported(branch)?;
        let is_last = index + 1 == branch_count;
        Ok((is_last, self.allocate_case_branch_blocks(is_last)))
    }

    fn finish_case_branch(
        &mut self,
        entry: BackendBlockId,
        current: BackendBlockId,
        else_block: BackendBlockId,
        span: SourceSpan,
        instructions: Vec<BackendInstruction>,
        entry_instructions: Option<Vec<BackendInstruction>>,
    ) -> Result<BackendBlock, BackendContractError> {
        self.blocks.push(BackendBlock {
            parameters: Vec::new(),
            id: else_block,
            instructions: vec![BackendInstruction::Trap {
                reason: "non-exhaustive enum case".to_owned(),
                span,
            }],
        });
        finish_case_blocks(self, entry, current, instructions, entry_instructions)
    }

    fn allocate_case_branch_blocks(
        &mut self,
        is_last: bool,
    ) -> (BackendBlockId, Option<BackendBlockId>, BackendBlockId) {
        let matched_id = self.allocate_block();
        let following_id = (!is_last).then(|| self.allocate_block());
        let else_block = following_id.unwrap_or_else(|| self.allocate_block());
        (matched_id, following_id, else_block)
    }

    fn insert_case_branch_body(
        &mut self,
        id: BackendBlockId,
        branch: &crate::ast::CaseBranch,
        span: SourceSpan,
        subject: BackendValueId,
        mode: CaseMode,
        fallthrough: Option<BackendBlockId>,
    ) -> Result<(), BackendContractError> {
        let branch_block_start = self.blocks.len();
        let body = lower_case_branch_body(self, id, branch, span, subject, mode, fallthrough)?;
        self.blocks.insert(branch_block_start, body);
        Ok(())
    }
}

fn ensure_case_branch_supported(
    branch: &crate::ast::CaseBranch,
) -> Result<(), BackendContractError> {
    branch
        .guard
        .is_none()
        .then_some(())
        .ok_or_else(|| unsupported(branch.span, "guarded enum case branch"))
}

fn finish_case_blocks<P: BackendExpressionProvider>(
    lowering: &mut BodyLowering<'_, P>,
    entry: BackendBlockId,
    current: BackendBlockId,
    instructions: Vec<BackendInstruction>,
    entry_instructions: Option<Vec<BackendInstruction>>,
) -> Result<BackendBlock, BackendContractError> {
    if let Some(entry_instructions) = entry_instructions {
        lowering.blocks.push(BackendBlock { parameters: Vec::new(), id: current, instructions });
        return Ok(BackendBlock {
            parameters: Vec::new(),
            id: entry,
            instructions: entry_instructions,
        });
    }
    Ok(BackendBlock { parameters: Vec::new(), id: current, instructions })
}
fn lower_case_branch_body<P: BackendExpressionProvider>(
    lowering: &mut BodyLowering<'_, P>,
    id: BackendBlockId,
    branch: &crate::ast::CaseBranch,
    span: SourceSpan,
    subject: BackendValueId,
    mode: CaseMode,
    fallthrough: Option<BackendBlockId>,
) -> Result<BackendBlock, BackendContractError> {
    lowering.provider.enter_scope(branch.span);
    let mut body_prefix = Vec::new();
    let binding_result = if matches!(branch.pattern, Pattern::Variant { .. }) {
        lowering.provider.bind_enum_variant_payload(
            subject,
            branch_variant(branch),
            branch_payload(branch),
            mode,
            branch.span,
            &mut body_prefix,
        )
    } else {
        Ok(())
    };
    let nested_block_start = lowering.blocks.len();
    match binding_result.and_then(|_| case_return_block(lowering, id, branch, span, fallthrough)) {
        Ok(mut body) => {
            body.instructions.splice(0..0, body_prefix);
            lowering.provider.distribute_scope_cleanup(
                branch.span,
                &mut lowering.blocks,
                nested_block_start,
            )?;
            lowering.provider.exit_scope(branch.span, &mut body.instructions)?;
            Ok(body)
        }
        Err(error) => {
            let mut discarded = Vec::new();
            let _ = lowering.provider.exit_scope(branch.span, &mut discarded);
            Err(error)
        }
    }
}
pub(super) fn lower_case_condition<P: BackendExpressionProvider>(
    lowering: &mut BodyLowering<'_, P>,
    subject: &Expr,
    branch: &crate::ast::CaseBranch,
    is_last: bool,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(BackendValueId, BackendValueId), BackendContractError> {
    match &branch.pattern {
        Pattern::Variant { variant, .. } => {
            let subject_value = lowering.provider.lower_expression(subject, instructions)?;
            let condition = lowering.provider.lower_enum_variant_condition_value(
                subject_value,
                variant,
                branch.span,
                instructions,
            )?;
            Ok((subject_value, condition))
        }
        Pattern::Wildcard { .. } if is_last => {
            let subject_value = lowering.provider.lower_expression(subject, instructions)?;
            let condition = lowering.provider.lower_expression(
                &Expr::BoolLiteral { value: true, span: branch.span },
                instructions,
            )?;
            Ok((subject_value, condition))
        }
        Pattern::Literal { value, .. } => {
            let subject_value = lowering.provider.lower_expression(subject, instructions)?;
            let expected = literal_case_expression(
                value,
                lowering.provider.value_type(subject_value),
                branch.span,
            )?;
            let expected_value = lowering.provider.lower_expression(&expected, instructions)?;
            let condition = lowering.provider.allocate_value(BackendValueType::Bool);
            instructions.push(BackendInstruction::IntegerCompare {
                result: condition,
                operation: super::super::BackendIntegerCompareOp::Equals,
                left: subject_value,
                right: expected_value,
                signed: true,
                span: branch.span,
            });
            Ok((subject_value, condition))
        }
        _ => Err(unsupported(branch.span, "enum case payload or pattern")),
    }
}

fn literal_case_expression(
    pattern: &LiteralPattern,
    subject_type: Option<BackendValueType>,
    span: SourceSpan,
) -> Result<Expr, BackendContractError> {
    match pattern {
        LiteralPattern::Bool(value) => Ok(Expr::BoolLiteral { value: *value, span }),
        LiteralPattern::Integer(value) => {
            let Some(BackendValueType::Integer { signed, width }) = subject_type else {
                return Err(unsupported(span, "integer case pattern requires an integer subject"));
            };
            Ok(Expr::Integer {
                value: value.clone(),
                suffix: Some(integer_suffix(signed, width).to_owned()),
                span,
            })
        }
    }
}

fn integer_suffix(signed: bool, width: u8) -> &'static str {
    match (signed, width) {
        (true, 8) => "i8",
        (true, 16) => "i16",
        (true, 32) => "i32",
        (true, 64) => "i64",
        (false, 8) => "u8",
        (false, 16) => "u16",
        (false, 32) => "u32",
        (false, 64) => "u64",
        _ => "i32",
    }
}
fn branch_variant(branch: &crate::ast::CaseBranch) -> &str {
    match &branch.pattern {
        Pattern::Variant { variant, .. } => variant,
        Pattern::Wildcard { .. } | Pattern::Literal { .. } => "_",
    }
}

fn branch_payload(branch: &crate::ast::CaseBranch) -> Option<&VariantPayload> {
    match &branch.pattern {
        Pattern::Variant { payload, .. } => Some(payload),
        Pattern::Wildcard { .. } | Pattern::Literal { .. } => None,
    }
}

fn case_return_block<P: BackendExpressionProvider>(
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
            if fallthrough.is_none()
                && let Some(Stmt::Expression { expression, span }) = block.statements.last()
            {
                let mut statements = block.statements[..block.statements.len() - 1].to_vec();
                statements.push(Stmt::Return { value: Some(expression.clone()), span: *span });
                let block = Block { statements, span: block.span };
                return lowering.lower_block(id, &block, fallthrough);
            }
            lowering.lower_block(id, block, fallthrough)
        }
    }
}
