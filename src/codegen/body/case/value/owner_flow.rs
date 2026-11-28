use std::collections::BTreeMap;

use super::super::super::super::BackendBlockParameter;
use super::super::super::contract::unsupported;
use super::super::super::loops::CarriedBinding;
use super::values::CaseOwnerBranchesInput;
use super::{
    BackendBlock, BackendBlockId, BackendContractError, BackendExpressionProvider,
    BackendInstruction, BackendValueId, BodyLowering, CaseBranchValue,
};
use crate::ast::Expr;
use crate::codegen::BackendValueType;
use crate::lexer::SourceSpan;

impl<P: BackendExpressionProvider> BodyLowering<'_, P> {
    pub(super) fn case_owner_initial_values(
        &self,
        carried: &[CarriedBinding],
    ) -> BTreeMap<String, BackendValueId> {
        carried
            .iter()
            .map(|binding| (binding.name.clone(), binding.value))
            .collect::<BTreeMap<_, _>>()
    }

    pub(super) fn case_result_type(
        &self,
        branch_values: &[CaseBranchValue],
        span: SourceSpan,
    ) -> Result<BackendValueType, BackendContractError> {
        let Some(first_value) = branch_values.first().map(|branch| branch.value) else {
            return Err(unsupported(span, "empty enum case"));
        };
        self.provider
            .value_type(first_value)
            .ok_or_else(|| unsupported(span, "case result type is unavailable"))
    }

    pub(super) fn advance_case_owner_branch(
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

    /// Lowers a `case` that is the value of an arm of another `case`. Its
    /// merge block is where the outer arm hands the value on, so the outer
    /// arm's scope (`outer_scope`) ends there (ADR-0125).
    pub(super) fn lower_nested_case_value(
        &mut self,
        id: BackendBlockId,
        expression: &Expr,
        span: SourceSpan,
        instructions: Vec<BackendInstruction>,
        outer_scope: SourceSpan,
    ) -> Result<(BackendBlock, BackendBlockId, BackendValueId), BackendContractError> {
        let Expr::Case { mode, subject, branches, .. } = expression else {
            return Err(unsupported(span, "nested enum case expected"));
        };
        let empty_carried: [CarriedBinding; 0] = [];
        let empty_initial_values: BTreeMap<String, BackendValueId> = BTreeMap::new();
        let lowered = self.lower_case_owner_branches(CaseOwnerBranchesInput {
            id,
            mode: *mode,
            subject,
            branches,
            span,
            carried: &empty_carried,
            initial_values: &empty_initial_values,
            instructions,
        })?;
        let Some(first_value) = lowered.branch_values.first().map(|branch| branch.value) else {
            return Err(unsupported(span, "empty nested enum case"));
        };
        let result_type = self
            .provider
            .value_type(first_value)
            .ok_or_else(|| unsupported(span, "nested case result type is unavailable"))?;
        let result = self.provider.allocate_value(result_type);
        self.provider.copy_value_metadata(first_value, result);
        let mut merge_instructions = Vec::new();
        self.provider.emit_scope_end(outer_scope, &mut merge_instructions);
        self.blocks.push(BackendBlock {
            parameters: vec![BackendBlockParameter { value: result, ty: result_type, span }],
            id: lowered.merge_id,
            instructions: merge_instructions,
        });
        self.attach_case_value_branches(
            lowered.branch_values,
            lowered.merge_id,
            result_type,
            &empty_carried,
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
