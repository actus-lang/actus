// Loop statement lowering for body control flow.

use super::loops::{replace_jump_arguments, replace_jump_arguments_in_block};
use super::{
    BackendBlock, BackendBlockId, BackendBlockParameter, BackendContractError, BackendEdgeArgument,
    BackendInstruction, BackendValueId, BodyLowering, LoopTargets,
};
use crate::ast::Block;

struct LoopState {
    header_id: BackendBlockId,
    body_id: BackendBlockId,
    continuation_id: BackendBlockId,
    header: BackendBlock,
    body_values: Vec<BackendValueId>,
    exit_values: Vec<BackendValueId>,
}

struct LoopBodyInput<'a> {
    body_id: BackendBlockId,
    header_id: BackendBlockId,
    continuation_id: BackendBlockId,
    nested: &'a Block,
    carried: &'a [super::loops::CarriedBinding],
    body_values: &'a [BackendValueId],
    span: crate::lexer::SourceSpan,
}

impl<P: super::BackendExpressionProvider> BodyLowering<'_, P> {
    pub(super) fn lower_loop_statement(
        &mut self,
        id: BackendBlockId,
        block: &Block,
        index: usize,
        nested: &Block,
        fallthrough: Option<BackendBlockId>,
        mut instructions: Vec<BackendInstruction>,
    ) -> Result<BackendBlock, BackendContractError> {
        let carried = self.collect_carried_bindings(nested, nested.span)?;
        let state = self.prepare_loop(&carried, nested.span);
        instructions.push(BackendInstruction::Jump {
            target: state.header_id,
            arguments: carried
                .iter()
                .map(|binding| BackendEdgeArgument {
                    value: binding.value,
                    ty: binding.ty,
                    span: nested.span,
                })
                .collect(),
            span: nested.span,
        });
        let (body, _) = self.lower_loop_body(LoopBodyInput {
            body_id: state.body_id,
            header_id: state.header_id,
            continuation_id: state.continuation_id,
            nested,
            carried: &carried,
            body_values: &state.body_values,
            span: nested.span,
        })?;
        for (binding, value) in carried.iter().zip(&state.exit_values) {
            self.provider.set_binding_value(&binding.name, *value);
        }
        let continuation =
            Block { statements: block.statements[index + 1..].to_vec(), span: block.span };
        let mut continuation_block =
            self.lower_block_in_scope(state.continuation_id, &continuation, fallthrough)?;
        continuation_block.parameters = state
            .exit_values
            .iter()
            .zip(&carried)
            .map(|(value, binding)| BackendBlockParameter {
                value: *value,
                ty: binding.ty,
                span: nested.span,
            })
            .collect();
        self.blocks.push(state.header);
        self.blocks.push(body);
        self.blocks.push(continuation_block);
        Ok(BackendBlock { parameters: Vec::new(), id, instructions })
    }

    fn prepare_loop(
        &mut self,
        carried: &[super::loops::CarriedBinding],
        span: crate::lexer::SourceSpan,
    ) -> LoopState {
        let header_id = self.allocate_block();
        let body_id = self.allocate_block();
        let continuation_id = self.allocate_block();
        let header_values = carried
            .iter()
            .map(|binding| self.provider.allocate_value(binding.ty))
            .collect::<Vec<_>>();
        let body_values = carried
            .iter()
            .map(|binding| self.provider.allocate_value(binding.ty))
            .collect::<Vec<_>>();
        let exit_values = carried
            .iter()
            .map(|binding| self.provider.allocate_value(binding.ty))
            .collect::<Vec<_>>();
        let header = BackendBlock {
            parameters: header_values
                .iter()
                .zip(carried)
                .map(|(value, binding)| BackendBlockParameter {
                    value: *value,
                    ty: binding.ty,
                    span,
                })
                .collect(),
            id: header_id,
            instructions: vec![BackendInstruction::Jump {
                target: body_id,
                arguments: header_values
                    .iter()
                    .zip(carried)
                    .map(|(value, binding)| BackendEdgeArgument {
                        value: *value,
                        ty: binding.ty,
                        span,
                    })
                    .collect(),
                span,
            }],
        };
        LoopState { header_id, body_id, continuation_id, header, body_values, exit_values }
    }

    fn lower_loop_body(
        &mut self,
        input: LoopBodyInput<'_>,
    ) -> Result<(BackendBlock, Vec<BackendEdgeArgument>), BackendContractError> {
        let LoopBodyInput {
            body_id,
            header_id,
            continuation_id,
            nested,
            carried,
            body_values,
            span,
        } = input;
        self.loop_targets.push(LoopTargets {
            continue_target: header_id,
            break_target: continuation_id,
            continue_values: carried.iter().map(|binding| binding.name.clone()).collect(),
            break_values: carried.iter().map(|binding| binding.name.clone()).collect(),
        });
        self.provider.enter_loop();
        for (binding, value) in carried.iter().zip(body_values) {
            self.provider.set_binding_value(&binding.name, *value);
        }
        self.provider.enter_scope(span);
        let nested_block_start = self.blocks.len();
        let body_result = self.lower_block_in_scope(body_id, nested, Some(header_id));
        self.provider.exit_loop();
        self.loop_targets.pop();
        let mut body = match body_result {
            Ok(body) => body,
            Err(error) => {
                let mut discarded = Vec::new();
                let _ = self.provider.exit_scope(span, &mut discarded);
                return Err(error);
            }
        };
        let continue_arguments = self.loop_edge_arguments(
            &carried.iter().map(|binding| binding.name.clone()).collect::<Vec<_>>(),
            span,
        )?;
        self.provider.exit_scope(span, &mut body.instructions)?;
        body.parameters = body_values
            .iter()
            .zip(carried)
            .map(|(value, binding)| BackendBlockParameter { value: *value, ty: binding.ty, span })
            .collect();
        body = replace_jump_arguments(body, header_id, continue_arguments.clone());
        for block in self.blocks.iter_mut().skip(nested_block_start) {
            replace_jump_arguments_in_block(block, header_id, &continue_arguments);
        }
        Ok((body, continue_arguments))
    }
}
