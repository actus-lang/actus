use super::super::body_contract::unsupported;
use super::{
    BackendBlock, BackendBlockId, BackendContractError, BackendExpressionProvider,
    BackendInstruction, BackendValueId, BodyLowering,
};
use crate::ast::{Block, CaseBody, CaseMode, Expr, Pattern, Stmt, VariantPayload};
use crate::lexer::SourceSpan;
impl<P: BackendExpressionProvider> BodyLowering<'_, P> {
    pub(super) fn lower_case_return(
        &mut self,
        id: BackendBlockId,
        expression: &Expr,
        span: SourceSpan,
        mut instructions: Vec<BackendInstruction>,
    ) -> Result<BackendBlock, BackendContractError> {
        let Expr::Case { mode, subject, branches, .. } = expression else {
            return Err(unsupported(span, "enum case expression expected"));
        };
        let mut current_id = id;
        let mut entry_instructions = None;
        for (index, branch) in branches.iter().enumerate() {
            ensure_case_branch_supported(branch)?;
            let matched_id = self.allocate_block();
            let is_last = index + 1 == branches.len();
            let following_id = (!is_last).then(|| self.allocate_block());
            let (subject_value, matched) =
                lower_case_condition(self, subject, branch, is_last, &mut instructions)?;
            let else_block = following_id.unwrap_or_else(|| self.allocate_block());
            instructions.push(BackendInstruction::Branch {
                condition: matched,
                then_block: matched_id,
                else_block,
                span: branch.span,
            });
            let body = lower_case_branch_body(
                self,
                matched_id,
                branch,
                span,
                subject_value,
                *mode,
                &mut instructions,
            )?;
            self.blocks.push(body);
            if let Some(following_id) = following_id {
                if entry_instructions.is_none() {
                    entry_instructions = Some(std::mem::take(&mut instructions));
                } else {
                    self.blocks.push(BackendBlock { id: current_id, instructions });
                    instructions = Vec::new();
                }
                current_id = following_id;
            } else {
                self.blocks.push(BackendBlock {
                    id: else_block,
                    instructions: vec![BackendInstruction::Trap {
                        reason: "non-exhaustive enum case".to_owned(),
                        span,
                    }],
                });
                return finish_case_blocks(self, id, current_id, instructions, entry_instructions);
            }
        }
        Err(unsupported(span, "empty enum case"))
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
        lowering.blocks.push(BackendBlock { id: current, instructions });
        return Ok(BackendBlock { id: entry, instructions: entry_instructions });
    }
    Ok(BackendBlock { id: current, instructions })
}
fn lower_case_branch_body<P: BackendExpressionProvider>(
    lowering: &mut BodyLowering<'_, P>,
    id: BackendBlockId,
    branch: &crate::ast::CaseBranch,
    span: SourceSpan,
    subject: BackendValueId,
    mode: CaseMode,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendBlock, BackendContractError> {
    lowering.provider.enter_scope(branch.span);
    let binding_result = if matches!(branch.pattern, Pattern::Variant { .. }) {
        lowering.provider.bind_enum_variant_payload(
            subject,
            branch_variant(branch),
            branch_payload(branch),
            mode,
            branch.span,
            instructions,
        )
    } else {
        Ok(())
    };
    match binding_result.and_then(|_| case_return_block(lowering, id, branch, span)) {
        Ok(mut body) => {
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
fn lower_case_condition<P: BackendExpressionProvider>(
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
        _ => Err(unsupported(branch.span, "enum case payload or pattern")),
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
) -> Result<BackendBlock, BackendContractError> {
    let CaseBody::Expression(expression) = &branch.body else {
        return Err(unsupported(branch.span, "block-bodied enum case branch"));
    };
    let block = Block {
        statements: vec![Stmt::Return { value: Some(*expression.clone()), span }],
        span: branch.span,
    };
    lowering.lower_block(id, &block, None)
}
