use std::collections::BTreeMap;

use super::super::super::body_contract::unsupported;
use super::super::loops::CarriedBinding;
use super::{
    BackendBlock, BackendBlockId, BackendContractError, BackendExpressionProvider,
    BackendInstruction, BackendValueId, BodyLowering, CaseBranchValue,
};
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
}
