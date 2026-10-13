use super::super::super::body_contract::unsupported;
use super::super::super::{
    BackendContractError, BackendInstruction, BackendValueId, BackendValueType,
};
use super::super::BodyLowering;
use super::super::loop_support::range_binding_type;
use super::{LoopBlocks, LoopValues, initial_loop_arguments};
use crate::ast::{Block, Expr, ForBinding};
use crate::lexer::SourceSpan;

pub(super) struct RangeSetup {
    pub(super) index_type: BackendValueType,
    pub(super) end_value: BackendValueId,
    pub(super) carried: Vec<super::CarriedBinding>,
    pub(super) ids: LoopValues,
    pub(super) blocks: LoopBlocks,
    pub(super) instructions: Vec<BackendInstruction>,
}

pub(super) struct RangeBodyInput<'a> {
    pub(super) body_id: super::BackendBlockId,
    pub(super) increment_id: super::BackendBlockId,
    pub(super) continuation_id: super::BackendBlockId,
    pub(super) binding: &'a ForBinding,
    pub(super) body: &'a Block,
    pub(super) ids: &'a LoopValues,
    pub(super) carried: &'a [super::CarriedBinding],
    pub(super) span: SourceSpan,
}

impl<P: super::super::BackendExpressionProvider> BodyLowering<'_, P> {
    pub(super) fn prepare_range_loop(
        &mut self,
        binding: &ForBinding,
        start: &Expr,
        end: &Expr,
        body: &Block,
        span: SourceSpan,
        mut instructions: Vec<BackendInstruction>,
    ) -> Result<RangeSetup, BackendContractError> {
        if !self.provider.supports_range_loops() {
            return Err(unsupported(span, "range loop"));
        }
        let index_type = range_binding_type(binding, span)?;
        let start_value = self.provider.lower_expression(start, &mut instructions)?;
        let end_value = self.provider.lower_expression(end, &mut instructions)?;
        let carried = self.collect_carried_bindings(body, span)?;
        let ids = self.allocate_loop_values(index_type, &carried);
        let blocks = self.allocate_loop_blocks();
        instructions.push(BackendInstruction::Jump {
            target: blocks.header,
            arguments: initial_loop_arguments(start_value, index_type, &carried, span),
            span,
        });
        Ok(RangeSetup { index_type, end_value, carried, ids, blocks, instructions })
    }
}
