use std::collections::{BTreeMap, BTreeSet};

use cranelift_codegen::ir::Block;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::Module;
use cranelift_object::ObjectModule;

use super::super::aggregate::{Declaration, initialize_values};
use super::super::native::BackendEmitError;
use super::{declarations, dispatch};
use crate::codegen::{BackendFunction, BackendInstruction, BackendValueId};

pub(super) fn define_function(
    module: &mut ObjectModule,
    function: &BackendFunction,
    declarations: &BTreeMap<String, Declaration>,
) -> Result<(), BackendEmitError> {
    if function.blocks.is_empty() {
        return Err(BackendEmitError(
            "Cranelift neutral contract function requires one or more blocks".to_owned(),
        ));
    }
    let declaration = declarations
        .get(function.symbol.as_str())
        .cloned()
        .ok_or_else(|| BackendEmitError("Cranelift function declaration is missing".to_owned()))?;
    let mut context = module.make_context();
    context.func.signature = declarations::signature(
        module,
        &function.parameters,
        &function.return_value,
        function.hosted_entry_return,
        false,
    )?;
    let mut function_context = FunctionBuilderContext::new();
    lower_function_body(module, function, declarations, &mut context, &mut function_context)?;
    module.define_function(declaration.id, &mut context).map_err(|error| {
        let calls = function
            .blocks
            .iter()
            .flat_map(|block| block.instructions.iter())
            .filter_map(|instruction| match instruction {
                BackendInstruction::Call { callee, arguments, .. } => {
                    let declaration = declarations.get(callee.as_str());
                    Some(format!(
                        "{} args={} declared={} return_slot={}",
                        callee.as_str(),
                        arguments.len(),
                        declaration.map_or(0, |value| value.parameter_fragments.len()),
                        declaration.is_some_and(|value| value.return_slot.is_some()),
                    ))
                }
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(", ");
        BackendEmitError(format!(
            "Cranelift verifier failed for native function `{}`: {error:?}; calls: {calls}",
            function.symbol.as_str(),
        ))
    })?;
    module.clear_context(&mut context);
    Ok(())
}

fn lower_function_body(
    module: &mut ObjectModule,
    function: &BackendFunction,
    declarations: &BTreeMap<String, Declaration>,
    context: &mut cranelift_codegen::Context,
    function_context: &mut FunctionBuilderContext,
) -> Result<(), BackendEmitError> {
    let mut builder = FunctionBuilder::new(&mut context.func, function_context);
    let blocks = create_blocks(&mut builder, function);
    let entry = blocks.get(&function.entry).copied().ok_or_else(|| {
        BackendEmitError("Cranelift neutral contract entry block is missing".to_owned())
    })?;
    for block in &function.blocks {
        if block.id != function.entry {
            append_block_parameters(&mut builder, &blocks, module, block)?;
        }
    }
    builder.switch_to_block(entry);
    let (return_slot, mut values, mut aggregate_values) =
        initialize_values(&mut builder, entry, function)?;
    let return_value_ids = function
        .blocks
        .iter()
        .flat_map(|block| block.instructions.iter())
        .filter_map(|instruction| match instruction {
            BackendInstruction::Return { value: Some(value), .. } => Some(*value),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    append_block_parameters_for_entry(&mut builder, &entry, module, function)?;
    bind_block_parameters(&builder, &blocks, function, &mut values)?;
    let mut state = dispatch::LoweringState {
        declarations,
        blocks: &blocks,
        return_slot,
        return_value_ids: &return_value_ids,
        values: &mut values,
        aggregate_values: &mut aggregate_values,
        function,
    };
    lower_blocks(module, &mut builder, &mut state)?;
    builder.seal_all_blocks();
    builder.finalize(module.isa().frontend_config());
    Ok(())
}

fn lower_blocks(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    state: &mut dispatch::LoweringState<'_>,
) -> Result<(), BackendEmitError> {
    let mut remaining = state.function.blocks.iter().collect::<Vec<_>>();
    while !remaining.is_empty() {
        let Some(index) = remaining.iter().position(|block| {
            block_inputs_are_available(block, state.values, state.aggregate_values)
        }) else {
            return Err(BackendEmitError(
                "Cranelift neutral contract contains an unresolved block value dependency"
                    .to_owned(),
            ));
        };
        let block = remaining.remove(index);
        let native_block = state.blocks.get(&block.id).copied().ok_or_else(|| {
            BackendEmitError("Cranelift neutral contract block is missing".to_owned())
        })?;
        builder.switch_to_block(native_block);
        for instruction in &block.instructions {
            dispatch::lower_instruction(module, builder, instruction, state)?;
        }
    }
    Ok(())
}

fn block_inputs_are_available(
    block: &crate::codegen::BackendBlock,
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
    aggregate_values: &BTreeMap<BackendValueId, Vec<cranelift_codegen::ir::Value>>,
) -> bool {
    let mut available =
        values.keys().chain(aggregate_values.keys()).copied().collect::<BTreeSet<_>>();
    for instruction in &block.instructions {
        if instruction_inputs(instruction).iter().any(|value| !available.contains(value)) {
            return false;
        }
        if let Some(result) = instruction_result(instruction) {
            available.insert(result);
        }
    }
    true
}

fn instruction_result(instruction: &BackendInstruction) -> Option<BackendValueId> {
    match instruction {
        BackendInstruction::Constant { result, .. }
        | BackendInstruction::FloatConstant { result, .. }
        | BackendInstruction::StringLiteral { result, .. }
        | BackendInstruction::IntegerBinary { result, .. }
        | BackendInstruction::IntegerCast { result, .. }
        | BackendInstruction::FloatCast { result, .. }
        | BackendInstruction::IntegerCompare { result, .. }
        | BackendInstruction::FloatCompare { result, .. }
        | BackendInstruction::IntegerUnary { result, .. }
        | BackendInstruction::Call { result: Some(result), .. }
        | BackendInstruction::EnumDiscriminant { result, .. }
        | BackendInstruction::EnumPayload { result, .. }
        | BackendInstruction::EnumPayloadBinding { result, .. }
        | BackendInstruction::EnumConstruct { result, .. }
        | BackendInstruction::ArenaConstruct { result, .. }
        | BackendInstruction::ArenaPlace { result, .. }
        | BackendInstruction::AggregateConstruct { result, .. }
        | BackendInstruction::AggregateFieldLoad { result, .. }
        | BackendInstruction::PackedFieldLoad { result, .. }
        | BackendInstruction::ArrayElementLoad { result, .. }
        | BackendInstruction::ScalarLoanLoad { result, .. }
        | BackendInstruction::ScalarLoanMaterialize { result, .. } => Some(*result),
        _ => None,
    }
}

fn instruction_inputs(instruction: &BackendInstruction) -> Vec<BackendValueId> {
    match instruction {
        BackendInstruction::IntegerBinary { left, right, .. }
        | BackendInstruction::IntegerCompare { left, right, .. }
        | BackendInstruction::FloatCompare { left, right, .. } => vec![*left, *right],
        BackendInstruction::IntegerCast { source, .. }
        | BackendInstruction::FloatCast { source, .. }
        | BackendInstruction::IntegerUnary { operand: source, .. }
        | BackendInstruction::TrapIfZero { value: source, .. }
        | BackendInstruction::TrapIfShiftCountOutOfRange { count: source, .. }
        | BackendInstruction::TrapIfArrayIndexOutOfBounds { index: source, .. }
        | BackendInstruction::EnumDiscriminant { subject: source, .. }
        | BackendInstruction::EnumPayload { subject: source, .. }
        | BackendInstruction::EnumPayloadBinding { subject: source, .. } => vec![*source],
        BackendInstruction::TrapIfSignedDivisionOverflow { left, right, .. } => vec![*left, *right],
        BackendInstruction::EnumConstruct { payload, .. } => payload.iter().copied().collect(),
        BackendInstruction::Call { arguments, .. } => arguments.clone(),
        BackendInstruction::Branch { condition, then_arguments, else_arguments, .. } => {
            std::iter::once(*condition)
                .chain(then_arguments.iter().map(|argument| argument.value))
                .chain(else_arguments.iter().map(|argument| argument.value))
                .collect()
        }
        BackendInstruction::Jump { arguments, .. } => {
            arguments.iter().map(|argument| argument.value).collect()
        }
        BackendInstruction::Return { value, .. } => value.iter().copied().collect(),
        BackendInstruction::MemoryCopy { destination, source, .. } => vec![*destination, *source],
        BackendInstruction::ScalarLoanLoad { address, .. } => vec![*address],
        BackendInstruction::ScalarLoanStore { address, value, .. } => vec![*address, *value],
        BackendInstruction::ScalarLoanMaterialize { value, .. } => vec![*value],
        BackendInstruction::ArenaPlace { arena, value, .. } => vec![*arena, *value],
        BackendInstruction::AggregateConstruct { fields, .. } => {
            fields.iter().map(|field| field.value).collect()
        }
        BackendInstruction::AggregateFieldLoad { base, .. }
        | BackendInstruction::PackedFieldLoad { base, .. }
        | BackendInstruction::ArrayElementLoad { base, .. } => vec![*base],
        BackendInstruction::AggregateFieldStore { base, value, .. }
        | BackendInstruction::PackedFieldStore { base, value, .. } => vec![*base, *value],
        BackendInstruction::ArrayElementStore { base, index, value, .. } => {
            vec![*base, *index, *value]
        }
        BackendInstruction::Cleanup { operand, .. } => {
            operand.iter().map(|item| item.value).collect()
        }
        BackendInstruction::Constant { .. }
        | BackendInstruction::FloatConstant { .. }
        | BackendInstruction::StringLiteral { .. }
        | BackendInstruction::ArenaConstruct { .. }
        | BackendInstruction::Fail { .. }
        | BackendInstruction::Trap { .. } => Vec::new(),
    }
}

fn append_block_parameters(
    builder: &mut FunctionBuilder<'_>,
    blocks: &BTreeMap<crate::codegen::BackendBlockId, Block>,
    module: &ObjectModule,
    block: &crate::codegen::BackendBlock,
) -> Result<(), BackendEmitError> {
    let native_block = blocks.get(&block.id).copied().ok_or_else(|| {
        BackendEmitError("Cranelift neutral contract block is missing".to_owned())
    })?;
    for parameter in &block.parameters {
        let ty = block_parameter_type(module, parameter.ty)?.ok_or_else(|| {
            BackendEmitError("Cranelift block parameter cannot be unit".to_owned())
        })?;
        builder.append_block_param(native_block, ty);
    }
    Ok(())
}

fn append_block_parameters_for_entry(
    builder: &mut FunctionBuilder<'_>,
    entry: &Block,
    module: &ObjectModule,
    function: &BackendFunction,
) -> Result<(), BackendEmitError> {
    let block =
        function.blocks.iter().find(|block| function.entry == block.id).ok_or_else(|| {
            BackendEmitError("Cranelift neutral contract entry block is missing".to_owned())
        })?;
    for parameter in &block.parameters {
        let ty = block_parameter_type(module, parameter.ty)?.ok_or_else(|| {
            BackendEmitError("Cranelift block parameter cannot be unit".to_owned())
        })?;
        builder.append_block_param(*entry, ty);
    }
    Ok(())
}

fn block_parameter_type(
    module: &ObjectModule,
    ty: crate::codegen::BackendValueType,
) -> Result<Option<cranelift_codegen::ir::Type>, BackendEmitError> {
    if matches!(ty, crate::codegen::BackendValueType::Aggregate { .. }) {
        return Ok(Some(module.isa().pointer_type()));
    }
    super::declarations::scalar_type(module, ty)
}

fn bind_block_parameters(
    builder: &FunctionBuilder<'_>,
    blocks: &BTreeMap<crate::codegen::BackendBlockId, Block>,
    function: &BackendFunction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let entry_function_parameter_count = builder
        .block_params(
            blocks
                .get(&function.entry)
                .copied()
                .ok_or_else(|| BackendEmitError("Cranelift entry block is missing".to_owned()))?,
        )
        .len()
        .saturating_sub(
            function
                .blocks
                .iter()
                .find(|block| block.id == function.entry)
                .map_or(0, |block| block.parameters.len()),
        );
    for block in &function.blocks {
        let native_block = blocks.get(&block.id).copied().ok_or_else(|| {
            BackendEmitError("Cranelift neutral contract block is missing".to_owned())
        })?;
        let native_parameters = builder.block_params(native_block);
        let start = if block.id == function.entry { entry_function_parameter_count } else { 0 };
        for (parameter, value) in block.parameters.iter().zip(native_parameters.iter().skip(start))
        {
            values.insert(parameter.value, *value);
        }
    }
    Ok(())
}

fn create_blocks(
    builder: &mut FunctionBuilder<'_>,
    function: &BackendFunction,
) -> BTreeMap<crate::codegen::BackendBlockId, Block> {
    function.blocks.iter().map(|block| (block.id, builder.create_block())).collect()
}
