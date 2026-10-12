// Structured loop lowering carries the explicit block, binding, and edge state
// required to preserve Actus control-flow contracts.
use super::super::body_contract::unsupported;
use super::super::{
    BackendBlock, BackendBlockId, BackendBlockParameter, BackendContractError, BackendEdgeArgument,
    BackendInstruction, BackendIntegerBinaryOp, BackendIntegerCompareOp, BackendValueType,
};
use super::loop_support::{collect_assignment_names, index_type_signed, range_binding_type};
use super::{BodyLowering, LoopTargets};
use crate::ast::{Block, Expr, ForBinding, Stmt};
use crate::lexer::SourceSpan;

impl<P: super::BackendExpressionProvider> BodyLowering<'_, P> {
    pub(super) fn lower_for_statement(
        &mut self,
        id: BackendBlockId,
        block: &Block,
        index: usize,
        statement: &Stmt,
        fallthrough: Option<BackendBlockId>,
        instructions: Vec<BackendInstruction>,
    ) -> Result<BackendBlock, BackendContractError> {
        match statement {
            Stmt::ForRange { binding, start, end, body, span } => self.lower_for_range_statement(
                id,
                block,
                index,
                binding,
                start,
                end,
                body,
                *span,
                fallthrough,
                instructions,
            ),
            Stmt::ForArray { binding, collection, body, span } => self.lower_for_array_statement(
                id,
                block,
                index,
                binding,
                collection,
                body,
                *span,
                fallthrough,
                instructions,
            ),
            _ => Err(unsupported(SourceSpan::new(0, 0), "for loop")),
        }
    }

    pub(super) fn lower_for_array_statement(
        &mut self,
        id: BackendBlockId,
        block: &Block,
        index: usize,
        binding: &ForBinding,
        collection: &Expr,
        body: &Block,
        span: SourceSpan,
        fallthrough: Option<BackendBlockId>,
        mut instructions: Vec<BackendInstruction>,
    ) -> Result<BackendBlock, BackendContractError> {
        if !self.provider.supports_array_loops() {
            return Err(unsupported(span, "array loop"));
        }
        if !matches!(collection, Expr::Identifier { .. }) {
            return Err(unsupported(span, "array loop collection must be an identifier"));
        }
        let collection_value = self.provider.lower_expression(collection, &mut instructions)?;
        let capacity = self.provider.array_capacity(collection_value, span)?;
        let start = Expr::Integer { value: "0".to_owned(), suffix: Some("u32".to_owned()), span };
        let end =
            Expr::Integer { value: capacity.to_string(), suffix: Some("u32".to_owned()), span };
        self.lower_for_range_statement(
            id,
            block,
            index,
            binding,
            &start,
            &end,
            body,
            span,
            fallthrough,
            instructions,
        )
    }

    pub(super) fn lower_for_range_statement(
        &mut self,
        id: BackendBlockId,
        block: &Block,
        index: usize,
        binding: &ForBinding,
        start: &Expr,
        end: &Expr,
        body: &Block,
        span: SourceSpan,
        fallthrough: Option<BackendBlockId>,
        mut instructions: Vec<BackendInstruction>,
    ) -> Result<BackendBlock, BackendContractError> {
        if !self.provider.supports_range_loops() {
            return Err(unsupported(span, "range loop"));
        }
        let index_type = range_binding_type(binding, span)?;
        let start_value = self.provider.lower_expression(start, &mut instructions)?;
        let end_value = self.provider.lower_expression(end, &mut instructions)?;
        let carried = self.collect_carried_bindings(body, span)?;
        let ids = self.allocate_loop_values(index_type, &carried);
        let blocks = self.allocate_loop_blocks();
        let initial_arguments = initial_loop_arguments(start_value, index_type, &carried, span);
        instructions.push(BackendInstruction::Jump {
            target: blocks.header,
            arguments: initial_arguments,
            span,
        });
        let header = header_block(blocks, &ids, end_value, index_type, &carried, span);
        let body = self.lower_range_body(
            blocks.body,
            blocks.increment,
            blocks.continuation,
            binding,
            body,
            &ids,
            &carried,
            span,
        )?;
        let increment = increment_block(blocks, &ids, index_type, &carried, span);
        let continuation =
            Block { statements: block.statements[index + 1..].to_vec(), span: block.span };
        for (binding, value) in carried.iter().zip(&ids.exit_carried) {
            self.provider.set_binding_value(&binding.name, *value);
        }
        let mut continuation_block =
            self.lower_block_in_scope(blocks.continuation, &continuation, fallthrough)?;
        continuation_block.parameters = ids
            .exit_carried
            .iter()
            .zip(&carried)
            .map(|(value, binding)| BackendBlockParameter { value: *value, ty: binding.ty, span })
            .collect();
        self.blocks.push(header);
        self.blocks.push(body);
        self.blocks.push(increment);
        self.blocks.push(continuation_block);
        Ok(BackendBlock { parameters: Vec::new(), id, instructions })
    }

    fn allocate_loop_values(
        &mut self,
        ty: BackendValueType,
        carried: &[CarriedBinding],
    ) -> LoopValues {
        LoopValues {
            header: self.provider.allocate_value(ty),
            body: self.provider.allocate_value(ty),
            increment: self.provider.allocate_value(ty),
            one: self.provider.allocate_value(ty),
            next: self.provider.allocate_value(ty),
            condition: self.provider.allocate_value(BackendValueType::Bool),
            header_carried: carried
                .iter()
                .map(|binding| self.provider.allocate_value(binding.ty))
                .collect(),
            body_carried: carried
                .iter()
                .map(|binding| self.provider.allocate_value(binding.ty))
                .collect(),
            increment_carried: carried
                .iter()
                .map(|binding| self.provider.allocate_value(binding.ty))
                .collect(),
            exit_carried: carried
                .iter()
                .map(|binding| self.provider.allocate_value(binding.ty))
                .collect(),
        }
    }

    pub(super) fn collect_carried_bindings(
        &mut self,
        body: &Block,
        span: SourceSpan,
    ) -> Result<Vec<CarriedBinding>, BackendContractError> {
        let mut names = Vec::new();
        collect_assignment_names(body, &mut names);
        names.sort();
        names.dedup();
        names
            .into_iter()
            .filter_map(|name| {
                let expression = Expr::Identifier { name: name.clone(), span };
                let value = self.provider.lower_expression(&expression, &mut Vec::new()).ok()?;
                let ty = self.provider.value_type(value)?;
                Some(Ok(CarriedBinding { name, value, ty }))
            })
            .collect()
    }

    fn allocate_loop_blocks(&mut self) -> LoopBlocks {
        LoopBlocks {
            header: self.allocate_block(),
            body: self.allocate_block(),
            increment: self.allocate_block(),
            continuation: self.allocate_block(),
        }
    }

    fn lower_range_body(
        &mut self,
        body_id: BackendBlockId,
        increment_id: BackendBlockId,
        continuation_id: BackendBlockId,
        binding: &ForBinding,
        body: &Block,
        ids: &LoopValues,
        carried: &[CarriedBinding],
        span: SourceSpan,
    ) -> Result<BackendBlock, BackendContractError> {
        self.push_range_targets(increment_id, continuation_id, binding, carried);
        self.provider.enter_loop();
        for (binding, value) in carried.iter().zip(&ids.body_carried) {
            self.provider.set_binding_value(&binding.name, *value);
        }
        self.provider.enter_scope(body.span);
        let mut prefix = Vec::new();
        self.provider.bind_owner(
            &binding.name,
            ids.body,
            binding.ty.as_deref(),
            binding.span,
            &mut prefix,
        )?;
        let nested_block_start = self.blocks.len();
        let result = self.lower_block_in_scope(body_id, body, Some(increment_id));
        let mut lowered = match result {
            Ok(block) => block,
            Err(error) => {
                let mut discarded = Vec::new();
                let _ = self.provider.exit_scope(body.span, &mut discarded);
                self.provider.exit_loop();
                self.loop_targets.pop();
                return Err(error);
            }
        };
        let continue_names = std::iter::once(binding.name.clone())
            .chain(carried.iter().map(|binding| binding.name.clone()))
            .collect::<Vec<_>>();
        let continue_arguments = self.loop_edge_arguments(&continue_names, span)?;
        self.provider.exit_scope(body.span, &mut lowered.instructions)?;
        self.provider.exit_loop();
        self.loop_targets.pop();
        prefix.append(&mut lowered.instructions);
        lowered.instructions = prefix;
        let mut lowered = replace_jump_arguments(lowered, increment_id, continue_arguments.clone());
        for block in self.blocks.iter_mut().skip(nested_block_start) {
            replace_jump_arguments_in_block(block, increment_id, &continue_arguments);
        }
        lowered.parameters = body_parameters(ids.body, binding, ids, carried, span)?;
        Ok(lowered)
    }

    fn push_range_targets(
        &mut self,
        increment_id: BackendBlockId,
        continuation_id: BackendBlockId,
        binding: &ForBinding,
        carried: &[CarriedBinding],
    ) {
        self.loop_targets.push(LoopTargets {
            continue_target: increment_id,
            break_target: continuation_id,
            continue_values: std::iter::once(binding.name.clone())
                .chain(carried.iter().map(|binding| binding.name.clone()))
                .collect(),
            break_values: carried.iter().map(|binding| binding.name.clone()).collect(),
        });
    }
}

#[derive(Clone, Copy)]
struct LoopBlocks {
    header: BackendBlockId,
    body: BackendBlockId,
    increment: BackendBlockId,
    continuation: BackendBlockId,
}

#[derive(Clone)]
struct LoopValues {
    header: super::super::BackendValueId,
    body: super::super::BackendValueId,
    increment: super::super::BackendValueId,
    one: super::super::BackendValueId,
    next: super::super::BackendValueId,
    condition: super::super::BackendValueId,
    header_carried: Vec<super::super::BackendValueId>,
    body_carried: Vec<super::super::BackendValueId>,
    increment_carried: Vec<super::super::BackendValueId>,
    exit_carried: Vec<super::super::BackendValueId>,
}

pub(super) struct CarriedBinding {
    pub(super) name: String,
    pub(super) value: super::super::BackendValueId,
    pub(super) ty: BackendValueType,
}

fn initial_loop_arguments(
    start: super::super::BackendValueId,
    index_type: BackendValueType,
    carried: &[CarriedBinding],
    span: SourceSpan,
) -> Vec<BackendEdgeArgument> {
    std::iter::once(BackendEdgeArgument { value: start, ty: index_type, span })
        .chain(carried.iter().map(|binding| BackendEdgeArgument {
            value: binding.value,
            ty: binding.ty,
            span,
        }))
        .collect()
}

fn body_parameters(
    index: super::super::BackendValueId,
    binding: &ForBinding,
    ids: &LoopValues,
    carried: &[CarriedBinding],
    span: SourceSpan,
) -> Result<Vec<BackendBlockParameter>, BackendContractError> {
    Ok(std::iter::once(BackendBlockParameter {
        value: index,
        ty: range_binding_type(binding, span)?,
        span,
    })
    .chain(ids.body_carried.iter().zip(carried).map(|(value, binding)| BackendBlockParameter {
        value: *value,
        ty: binding.ty,
        span,
    }))
    .collect())
}

fn header_block(
    blocks: LoopBlocks,
    ids: &LoopValues,
    end_value: super::super::BackendValueId,
    index_type: BackendValueType,
    carried: &[CarriedBinding],
    span: SourceSpan,
) -> BackendBlock {
    let then_arguments =
        std::iter::once(BackendEdgeArgument { value: ids.header, ty: index_type, span })
            .chain(ids.header_carried.iter().zip(carried).map(|(value, binding)| {
                BackendEdgeArgument { value: *value, ty: binding.ty, span }
            }))
            .collect();
    let else_arguments = ids
        .header_carried
        .iter()
        .zip(carried)
        .map(|(value, binding)| BackendEdgeArgument { value: *value, ty: binding.ty, span })
        .collect();
    BackendBlock {
        id: blocks.header,
        parameters: std::iter::once(BackendBlockParameter {
            value: ids.header,
            ty: index_type,
            span,
        })
        .chain(
            ids.header_carried.iter().zip(carried).map(|(value, binding)| BackendBlockParameter {
                value: *value,
                ty: binding.ty,
                span,
            }),
        )
        .collect(),
        instructions: vec![
            BackendInstruction::IntegerCompare {
                result: ids.condition,
                operation: BackendIntegerCompareOp::LessThan,
                left: ids.header,
                right: end_value,
                signed: index_type_signed(index_type),
                span,
            },
            BackendInstruction::Branch {
                condition: ids.condition,
                then_block: blocks.body,
                else_block: blocks.continuation,
                then_arguments,
                else_arguments,
                span,
            },
        ],
    }
}

fn increment_block(
    blocks: LoopBlocks,
    values: &LoopValues,
    index_type: BackendValueType,
    carried: &[CarriedBinding],
    span: SourceSpan,
) -> BackendBlock {
    let parameters =
        std::iter::once(BackendBlockParameter { value: values.increment, ty: index_type, span })
            .chain(values.increment_carried.iter().zip(carried).map(|(value, binding)| {
                BackendBlockParameter { value: *value, ty: binding.ty, span }
            }))
            .collect();
    let arguments =
        std::iter::once(BackendEdgeArgument { value: values.next, ty: index_type, span })
            .chain(values.increment_carried.iter().zip(carried).map(|(value, binding)| {
                BackendEdgeArgument { value: *value, ty: binding.ty, span }
            }))
            .collect();
    BackendBlock {
        id: blocks.increment,
        parameters,
        instructions: vec![
            BackendInstruction::Constant { result: values.one, ty: index_type, value: 1, span },
            BackendInstruction::IntegerBinary {
                result: values.next,
                operation: BackendIntegerBinaryOp::Add,
                left: values.increment,
                right: values.one,
                ty: index_type,
                span,
            },
            BackendInstruction::Jump { target: blocks.header, arguments, span },
        ],
    }
}

pub(super) fn replace_jump_arguments(
    mut block: BackendBlock,
    target: BackendBlockId,
    edge_arguments: Vec<BackendEdgeArgument>,
) -> BackendBlock {
    if let Some(BackendInstruction::Jump { target: actual, arguments: current, .. }) =
        block.instructions.last_mut()
        && *actual == target
        && current.is_empty()
    {
        *current = edge_arguments;
    }
    block
}

pub(super) fn replace_jump_arguments_in_block(
    block: &mut BackendBlock,
    target: BackendBlockId,
    edge_arguments: &[BackendEdgeArgument],
) {
    if let Some(BackendInstruction::Jump { target: actual, arguments, .. }) =
        block.instructions.last_mut()
        && *actual == target
        && arguments.is_empty()
    {
        *arguments = edge_arguments.to_vec();
    }
}
