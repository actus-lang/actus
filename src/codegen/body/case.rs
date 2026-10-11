use crate::ast::{Block, CaseBody, CaseMode, Expr, Pattern, Stmt, VariantPayload};
use crate::lexer::SourceSpan;

use super::super::body_contract::unsupported;
use super::{
    BackendBlock, BackendBlockId, BackendContractError, BackendExpressionProvider,
    BackendInstruction, BackendValueId, BodyLowering,
};

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
        if *mode != CaseMode::Plain {
            return Err(unsupported(span, "non-plain enum case ownership mode"));
        }
        let mut current_id = id;
        let mut entry_instructions = None;
        for (index, branch) in branches.iter().enumerate() {
            if branch.guard.is_some() {
                return Err(unsupported(branch.span, "guarded enum case branch"));
            }
            let matched_id = self.allocate_block();
            let is_last = index + 1 == branches.len();
            let following_id = (!is_last).then(|| self.allocate_block());
            let matched = lower_case_condition(self, subject, branch, is_last, &mut instructions)?;
            let else_block = following_id.unwrap_or_else(|| self.allocate_block());
            instructions.push(BackendInstruction::Branch {
                condition: matched,
                then_block: matched_id,
                else_block,
                span: branch.span,
            });
            let body = case_return_block(self, matched_id, branch, span)?;
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
                if let Some(entry_instructions) = entry_instructions {
                    self.blocks.push(BackendBlock { id: current_id, instructions });
                    return Ok(BackendBlock { id, instructions: entry_instructions });
                }
                return Ok(BackendBlock { id: current_id, instructions });
            }
        }
        Err(unsupported(span, "empty enum case"))
    }
}

fn lower_case_condition<P: BackendExpressionProvider>(
    lowering: &mut BodyLowering<'_, P>,
    subject: &Expr,
    branch: &crate::ast::CaseBranch,
    is_last: bool,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    match &branch.pattern {
        Pattern::Variant { variant, payload, .. }
            if matches!(payload, VariantPayload::Unit)
                || matches!(payload, VariantPayload::Positional(items) if items.iter().all(|item| item.name == "_") ) =>
        {
            lowering.provider.lower_enum_variant_condition(
                subject,
                variant,
                branch.span,
                instructions,
            )
        }
        Pattern::Wildcard { .. } if is_last => lowering
            .provider
            .lower_expression(&Expr::BoolLiteral { value: true, span: branch.span }, instructions),
        _ => Err(unsupported(branch.span, "enum case payload or pattern")),
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
