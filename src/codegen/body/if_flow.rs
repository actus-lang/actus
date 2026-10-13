// Conditional control-flow lowering for body blocks.

use super::super::body_contract::{ensure_jump, terminates_without_continuation, unsupported};
use super::loop_support::collect_assignment_names;
use super::loops;
use super::{
    BackendBlock, BackendBlockId, BackendBlockParameter, BackendContractError, BackendEdgeArgument,
    BackendInstruction, BackendValueId, BodyLowering,
};
use crate::ast::{Block, Expr, IfBranch};
use crate::lexer::SourceSpan;
use std::collections::BTreeMap;

impl<P: super::BackendExpressionProvider> BodyLowering<'_, P> {
    pub(super) fn lower_if(
        &mut self,
        condition: &Expr,
        then_branch: &Block,
        else_branch: Option<&IfBranch>,
        span: SourceSpan,
        continuation_id: BackendBlockId,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<IfLowering, BackendContractError> {
        let carried = self.collect_if_carried_bindings(then_branch, else_branch, span)?;
        let initial = carried
            .iter()
            .filter_map(|binding| {
                self.provider
                    .binding_value(&binding.name)
                    .map(|value| (binding.name.clone(), value))
            })
            .collect::<BTreeMap<_, _>>();
        let condition = self.provider.lower_expression(condition, instructions)?;
        let then_id = self.allocate_block();
        let else_id = self.allocate_block();
        instructions.push(BackendInstruction::Branch {
            condition,
            then_block: then_id,
            else_block: else_id,
            then_arguments: Vec::new(),
            else_arguments: Vec::new(),
            span,
        });
        let branch_start = self.blocks.len();
        let (then_block, then_values, then_terminates) =
            self.lower_if_then_branch(then_id, then_branch, continuation_id, &carried, span)?;
        self.blocks.push(then_block);
        for block in self.blocks.iter_mut().skip(branch_start) {
            replace_if_edge_arguments(block, continuation_id, &carried, &then_values, span);
        }
        for (name, value) in &initial {
            self.provider.set_binding_value(name, *value);
        }
        let else_block =
            self.lower_if_else_block(else_id, else_branch, continuation_id, &carried, span)?;
        let branches_terminate = then_terminates
            && terminates_without_continuation(else_block.instructions.last(), continuation_id);
        self.blocks.push(else_block);
        let parameters = carried
            .iter()
            .map(|binding| {
                let value = self.provider.allocate_value(binding.ty);
                self.provider.set_binding_value(&binding.name, value);
                BackendBlockParameter { value, ty: binding.ty, span }
            })
            .collect();
        Ok(IfLowering { terminates: branches_terminate, parameters })
    }

    fn lower_if_then_branch(
        &mut self,
        id: BackendBlockId,
        branch: &Block,
        continuation_id: BackendBlockId,
        carried: &[loops::CarriedBinding],
        span: SourceSpan,
    ) -> Result<(BackendBlock, BTreeMap<String, BackendValueId>, bool), BackendContractError> {
        let mut lowered = self.lower_block(id, branch, Some(continuation_id))?;
        let values = carried
            .iter()
            .filter_map(|binding| {
                self.provider
                    .binding_value(&binding.name)
                    .map(|value| (binding.name.clone(), value))
            })
            .collect::<BTreeMap<_, _>>();
        let terminates =
            terminates_without_continuation(lowered.instructions.last(), continuation_id);
        replace_if_edge_arguments(&mut lowered, continuation_id, carried, &values, span);
        Ok((lowered, values, terminates))
    }

    fn lower_if_else_block(
        &mut self,
        id: BackendBlockId,
        branch: Option<&IfBranch>,
        continuation_id: BackendBlockId,
        carried: &[loops::CarriedBinding],
        span: SourceSpan,
    ) -> Result<BackendBlock, BackendContractError> {
        let mut lowered = self.lower_if_else_branch(id, branch, continuation_id, span)?;
        let values = carried
            .iter()
            .filter_map(|binding| {
                self.provider
                    .binding_value(&binding.name)
                    .map(|value| (binding.name.clone(), value))
            })
            .collect::<BTreeMap<_, _>>();
        replace_if_edge_arguments(&mut lowered, continuation_id, carried, &values, span);
        Ok(ensure_jump(lowered, continuation_id, span))
    }

    fn lower_if_else_branch(
        &mut self,
        id: BackendBlockId,
        branch: Option<&IfBranch>,
        continuation_id: BackendBlockId,
        span: SourceSpan,
    ) -> Result<BackendBlock, BackendContractError> {
        match branch {
            None => Ok(BackendBlock { parameters: Vec::new(), id, instructions: Vec::new() }),
            Some(IfBranch::Block(block)) => self.lower_block(id, block, Some(continuation_id)),
            Some(IfBranch::ElseIf(nested)) => {
                let Expr::If { condition, then_branch, else_branch, span: nested_span } =
                    nested.as_ref()
                else {
                    return Err(unsupported(span, "else-if expression"));
                };
                let mut instructions = Vec::new();
                let _ = self.lower_if(
                    condition,
                    then_branch,
                    else_branch.as_ref(),
                    *nested_span,
                    continuation_id,
                    &mut instructions,
                )?;
                Ok(BackendBlock { parameters: Vec::new(), id, instructions })
            }
        }
    }

    fn collect_if_carried_bindings(
        &mut self,
        then_branch: &Block,
        else_branch: Option<&IfBranch>,
        span: SourceSpan,
    ) -> Result<Vec<loops::CarriedBinding>, BackendContractError> {
        let mut names = Vec::new();
        collect_assignment_names(then_branch, &mut names);
        if let Some(IfBranch::Block(block)) = else_branch {
            collect_assignment_names(block, &mut names);
        }
        names.sort();
        names.dedup();
        if else_branch.is_none()
            && let Some(name) = names.first()
        {
            return Err(unsupported(
                span,
                format!(
                    "scalar assignment for `{name}` requires a local owner in the current lexical scope"
                ),
            ));
        }
        Ok(names
            .into_iter()
            .filter_map(|name| {
                let value = self.provider.binding_value(&name)?;
                let ty = self.provider.value_type(value)?;
                Some(loops::CarriedBinding { name, value, ty })
            })
            .collect())
    }

    pub(super) fn allocate_block(&mut self) -> BackendBlockId {
        let id = BackendBlockId(self.next_block);
        self.next_block += 1;
        id
    }
}

pub(super) struct IfLowering {
    pub(super) terminates: bool,
    pub(super) parameters: Vec<BackendBlockParameter>,
}

fn replace_if_edge_arguments(
    block: &mut BackendBlock,
    continuation: BackendBlockId,
    carried: &[loops::CarriedBinding],
    values: &BTreeMap<String, BackendValueId>,
    span: SourceSpan,
) {
    let arguments = carried
        .iter()
        .filter_map(|binding| {
            values.get(&binding.name).map(|value| BackendEdgeArgument {
                value: *value,
                ty: binding.ty,
                span,
            })
        })
        .collect::<Vec<_>>();
    loops::replace_jump_arguments_in_block(block, continuation, &arguments);
}
