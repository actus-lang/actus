// Block emission receives the complete native lowering context explicitly.
#![allow(clippy::too_many_arguments)]

use std::collections::BTreeMap;
mod aggregate;
mod arena;
mod arrays;
mod cleanup;
mod dispatch;
mod enums;
mod packed;
mod returns;
use super::aggregate::{
    Declaration, declaration, direct_aggregate_types, initialize_values, runtime_declaration,
};
use super::integer_ops::{
    lower_integer_binary, lower_integer_cast, lower_integer_compare, lower_integer_unary,
    lower_trap_if_array_index_out_of_bounds, lower_trap_if_shift_count_out_of_range,
    lower_trap_if_signed_division_overflow, lower_trap_if_zero,
};
use super::native::BackendEmitError;
use super::object::create_module;
use crate::codegen::{
    BackendFunction, BackendInstruction, BackendModule, BackendPassingMode, BackendValueId,
    BackendValueType,
};
use crate::configuration::NativeBackendConfiguration;
use crate::target::TargetSpec;
pub(super) use cleanup::LayoutCleanupProvider;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{AbiParam, Block, BlockArg, InstBuilder, MemFlagsData, Type, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module};
use cranelift_object::ObjectModule;
pub fn emit_contract_module_object(
    backend_module: &BackendModule,
    configuration: &NativeBackendConfiguration,
    target: &TargetSpec,
) -> Result<Vec<u8>, BackendEmitError> {
    backend_module.validate().map_err(|error| BackendEmitError(error.to_string()))?;
    if configuration.verifies_no_float_ir() {
        for function in &backend_module.functions {
            if function.blocks.iter().any(|block| {
                block.instructions.iter().any(|instruction| {
                    matches!(
                        instruction,
                        BackendInstruction::FloatConstant { .. }
                            | BackendInstruction::FloatCompare { .. }
                            | BackendInstruction::FloatCast { .. }
                    )
                })
            }) {
                return Err(BackendEmitError(format!(
                    "native function `{}` contains floating-point IR instructions: shared float instruction",
                    function.symbol.as_str()
                )));
            }
        }
    }
    let mut module = create_module(configuration, target)?;
    let declarations = declare_functions(&mut module, backend_module)?;
    for function in &backend_module.functions {
        define_function(&mut module, function, &declarations)?;
    }
    module.finish().emit().map_err(|error| BackendEmitError(error.to_string()))
}
fn declare_functions(
    module: &mut ObjectModule,
    backend_module: &BackendModule,
) -> Result<BTreeMap<String, Declaration>, BackendEmitError> {
    let mut declarations = BTreeMap::new();
    for external in &backend_module.externals {
        let signature = signature(
            module,
            &external.parameters,
            &external.return_value,
            false,
            external.runtime_pointer_return,
        )?;
        let id = module
            .declare_function(external.symbol.as_str(), Linkage::Import, &signature)
            .map_err(|error| BackendEmitError(error.to_string()))?;
        declarations.insert(
            external.symbol.as_str().to_owned(),
            runtime_declaration(module, id, external)?,
        );
    }
    for function in &backend_module.functions {
        let signature = signature(
            module,
            &function.parameters,
            &function.return_value,
            function.hosted_entry_return,
            false,
        )?;
        let id = module
            .declare_function(function.symbol.as_str(), Linkage::Export, &signature)
            .map_err(|error| BackendEmitError(error.to_string()))?;
        let lowered = declaration(module, id, &function.parameters, &function.return_value)?;
        declarations.insert(function.symbol.as_str().to_owned(), lowered);
    }
    cleanup::declare_cleanup_bridges(module, backend_module, &mut declarations)?;
    Ok(declarations)
}
fn signature(
    module: &ObjectModule,
    parameters: &[crate::codegen::BackendParameter],
    return_value: &crate::codegen::BackendReturn,
    hosted_entry_return: bool,
    runtime_pointer_return: bool,
) -> Result<cranelift_codegen::ir::Signature, BackendEmitError> {
    let mut signature = module.make_signature();
    for parameter in parameters {
        for ty in parameter_types(module, parameter)? {
            signature.params.push(AbiParam::new(ty));
        }
    }
    match &return_value.passing {
        crate::codegen::BackendReturnMode::Direct => {
            if hosted_entry_return {
                signature.returns.push(AbiParam::new(types::I32));
            } else if let Some(ty) = scalar_type(module, return_value.ty)? {
                signature.returns.push(AbiParam::new(ty));
            }
        }
        crate::codegen::BackendReturnMode::DirectAggregate { abi } => {
            for ty in direct_aggregate_types(module, abi, false)? {
                signature.returns.push(AbiParam::new(ty));
            }
        }
        crate::codegen::BackendReturnMode::ReturnSlot { layout } if runtime_pointer_return => {
            signature.returns.push(AbiParam::new(module.isa().pointer_type()));
            if matches!(
                return_value.ty,
                crate::codegen::BackendValueType::NichePointerOption { .. }
            ) {
                signature.params.insert(0, AbiParam::new(module.isa().pointer_type()));
            }
        }
        crate::codegen::BackendReturnMode::ReturnSlot { layout } => {
            if matches!(return_value.ty, BackendValueType::NichePointerOption { .. }) {
                signature.params.insert(0, AbiParam::new(module.isa().pointer_type()));
                return Ok(signature);
            }
            if !matches!(return_value.ty, BackendValueType::Aggregate { layout: value_layout } | BackendValueType::Region { layout: value_layout } | BackendValueType::Enum { layout: crate::codegen::BackendEnumLayout { layout: value_layout, .. } } if value_layout == *layout)
            {
                return Err(BackendEmitError(
                    "Cranelift return slot layout does not match the aggregate return type"
                        .to_owned(),
                ));
            }
            signature.params.insert(0, AbiParam::new(module.isa().pointer_type()));
        }
    }
    Ok(signature)
}
pub(super) fn parameter_types(
    module: &ObjectModule,
    parameter: &crate::codegen::BackendParameter,
) -> Result<Vec<Type>, BackendEmitError> {
    match &parameter.passing {
        BackendPassingMode::Direct => {
            scalar_type(module, parameter.ty)?.map(|ty| vec![ty]).ok_or_else(|| {
                BackendEmitError("Cranelift direct unit parameters are not supported".to_owned())
            })
        }
        BackendPassingMode::DirectAggregate { abi } => direct_aggregate_types(module, abi, true),
        BackendPassingMode::Indirect { layout }
        | BackendPassingMode::Borrowed { layout }
        | BackendPassingMode::ExclusiveLoan { layout }
        | BackendPassingMode::OwnershipTransfer { layout }
        | BackendPassingMode::RuntimeBridge { layout } => {
            layout.validate().map_err(|error| BackendEmitError(error.to_string()))?;
            match parameter.ty {
                BackendValueType::Pointer => Ok(vec![module.isa().pointer_type()]),
                BackendValueType::Aggregate { layout: value_layout }
                | BackendValueType::Region { layout: value_layout }
                | BackendValueType::Enum {
                    layout: crate::codegen::BackendEnumLayout { layout: value_layout, .. },
                } if value_layout == *layout => Ok(vec![module.isa().pointer_type()]),
                BackendValueType::Aggregate { .. }
                | BackendValueType::Region { .. }
                | BackendValueType::Enum { .. }
                | BackendValueType::NichePointerOption { .. } => Err(BackendEmitError(
                    "Cranelift aggregate parameter layout does not match passing layout".to_owned(),
                )),
                _ => Err(BackendEmitError(
                    "Cranelift indirect parameter must have an aggregate or Region type".to_owned(),
                )),
            }
        }
        BackendPassingMode::ExclusiveScalarLoan { .. } => Ok(vec![module.isa().pointer_type()]),
    }
}

fn scalar_type(
    module: &ObjectModule,
    ty: BackendValueType,
) -> Result<Option<Type>, BackendEmitError> {
    let result = match ty {
        BackendValueType::Unit => None,
        BackendValueType::Bool => Some(types::I8),
        BackendValueType::Integer { width: 8, .. } => Some(types::I8),
        BackendValueType::Integer { width: 16, .. } => Some(types::I16),
        BackendValueType::Integer { width: 32, .. } => Some(types::I32),
        BackendValueType::Integer { width: 64, .. } => Some(types::I64),
        BackendValueType::Pointer => Some(module.isa().pointer_type()),
        BackendValueType::Region { .. } => Some(module.isa().pointer_type()),
        BackendValueType::Enum { .. } => Some(module.isa().pointer_type()),
        BackendValueType::NichePointerOption { .. } => Some(module.isa().pointer_type()),
        BackendValueType::Float { width: 32 } => Some(types::F32),
        BackendValueType::Float { width: 64 } => Some(types::F64),
        BackendValueType::Integer { .. }
        | BackendValueType::Float { .. }
        | BackendValueType::Aggregate { .. } => {
            return Err(BackendEmitError(
                "Cranelift neutral contract slice supports scalar values only".to_owned(),
            ));
        }
    };
    Ok(result)
}

fn define_function(
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
    context.func.signature = signature(
        module,
        &function.parameters,
        &function.return_value,
        function.hosted_entry_return,
        false,
    )?;
    let mut function_context = FunctionBuilderContext::new();
    lower_function_body(module, function, declarations, &mut context, &mut function_context)?;
    module
        .define_function(declaration.id, &mut context)
        .map_err(|error| BackendEmitError(format!("Cranelift verifier failed: {error:?}")))?;
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
        .collect::<std::collections::BTreeSet<_>>();
    append_block_parameters_for_entry(&mut builder, &entry, module, function)?;
    bind_block_parameters(&builder, &blocks, function, &mut values)?;
    lower_blocks(
        module,
        &mut builder,
        function,
        declarations,
        &blocks,
        return_slot,
        &return_value_ids,
        &mut values,
        &mut aggregate_values,
    )?;
    builder.seal_all_blocks();
    builder.finalize(module.isa().frontend_config());
    Ok(())
}

fn lower_blocks(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    function: &BackendFunction,
    declarations: &BTreeMap<String, Declaration>,
    blocks: &BTreeMap<crate::codegen::BackendBlockId, Block>,
    return_slot: Option<cranelift_codegen::ir::Value>,
    return_value_ids: &std::collections::BTreeSet<BackendValueId>,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
    aggregate_values: &mut BTreeMap<BackendValueId, Vec<cranelift_codegen::ir::Value>>,
) -> Result<(), BackendEmitError> {
    let mut remaining = function.blocks.iter().collect::<Vec<_>>();
    while !remaining.is_empty() {
        let Some(index) = remaining
            .iter()
            .position(|block| block_inputs_are_available(block, values, aggregate_values))
        else {
            return Err(BackendEmitError(
                "Cranelift neutral contract contains an unresolved block value dependency"
                    .to_owned(),
            ));
        };
        let block = remaining.remove(index);
        let native_block = blocks.get(&block.id).copied().ok_or_else(|| {
            BackendEmitError("Cranelift neutral contract block is missing".to_owned())
        })?;
        builder.switch_to_block(native_block);
        for instruction in &block.instructions {
            dispatch::lower_instruction(
                module,
                builder,
                instruction,
                declarations,
                blocks,
                return_slot,
                return_value_ids,
                values,
                aggregate_values,
                function,
            )?;
        }
    }
    Ok(())
}

fn block_inputs_are_available(
    block: &crate::codegen::BackendBlock,
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
    aggregate_values: &BTreeMap<BackendValueId, Vec<cranelift_codegen::ir::Value>>,
) -> bool {
    let mut available = values
        .keys()
        .chain(aggregate_values.keys())
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
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
        let ty = scalar_type(module, parameter.ty)?.ok_or_else(|| {
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
        let ty = scalar_type(module, parameter.ty)?.ok_or_else(|| {
            BackendEmitError("Cranelift block parameter cannot be unit".to_owned())
        })?;
        builder.append_block_param(*entry, ty);
    }
    Ok(())
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

pub(super) fn lower_guard(
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    match instruction {
        BackendInstruction::TrapIfZero { .. } => lower_trap_if_zero(builder, instruction, values),
        BackendInstruction::TrapIfSignedDivisionOverflow { .. } => {
            lower_trap_if_signed_division_overflow(builder, instruction, values)
        }
        BackendInstruction::TrapIfShiftCountOutOfRange { .. } => {
            lower_trap_if_shift_count_out_of_range(builder, instruction, values)
        }
        BackendInstruction::TrapIfArrayIndexOutOfBounds { .. } => {
            lower_trap_if_array_index_out_of_bounds(builder, instruction, values)
        }
        _ => Err(BackendEmitError("Cranelift guard instruction is not supported".to_owned())),
    }
}

pub(super) fn lower_constant(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::Constant { result, ty, value, .. } = instruction else {
        return Err(BackendEmitError("Cranelift constant instruction is not supported".to_owned()));
    };
    let ty = scalar_type(module, *ty)?
        .ok_or_else(|| BackendEmitError("Cranelift constant cannot have unit type".to_owned()))?;
    values.insert(*result, builder.ins().iconst(ty, *value as i64));
    Ok(())
}

pub(super) fn lower_float_cast(
    _builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::FloatCast { result, source, source_type, target_type, .. } =
        instruction
    else {
        return Err(BackendEmitError(
            "Cranelift float cast instruction is not supported".to_owned(),
        ));
    };
    let source_value = values.get(source).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift float cast source {} is unavailable", source.0))
    })?;
    let _ = (result, source_type, target_type, source_value, values);
    Err(BackendEmitError(
        "Cranelift shared float conversion is not available for a non-literal value".to_owned(),
    ))
}

pub(super) fn lower_float_constant(
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::FloatConstant { result, ty, bits, .. } = instruction else {
        return Err(BackendEmitError(
            "Cranelift float constant instruction is not supported".to_owned(),
        ));
    };
    let value = match ty {
        BackendValueType::Float { width: 32 } => {
            builder.ins().f32const(f32::from_bits(*bits as u32))
        }
        BackendValueType::Float { width: 64 } => builder.ins().f64const(f64::from_bits(*bits)),
        _ => {
            return Err(BackendEmitError(
                "Cranelift float constant requires a float type".to_owned(),
            ));
        }
    };
    values.insert(*result, value);
    Ok(())
}

pub(super) fn lower_memory_copy(
    module: &ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::MemoryCopy { destination, source, size, alignment, .. } = instruction
    else {
        return Err(BackendEmitError("Cranelift memory-copy instruction expected".to_owned()));
    };
    let destination = values.get(destination).copied().ok_or_else(|| {
        BackendEmitError(format!(
            "Cranelift memory-copy destination {} is unavailable",
            destination.0
        ))
    })?;
    let source = values.get(source).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift memory-copy source {} is unavailable", source.0))
    })?;
    let alignment = u8::try_from(*alignment)
        .map_err(|_| BackendEmitError("Cranelift memory-copy alignment exceeds u8".to_owned()))?;
    builder.emit_small_memory_copy(
        module.isa().frontend_config(),
        destination,
        source,
        u64::from(*size),
        alignment,
        alignment,
        false,
        MemFlagsData::new(),
    );
    Ok(())
}

pub(super) fn lower_scalar_loan(
    module: &ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    match instruction {
        BackendInstruction::ScalarLoanLoad { result, address, ty, .. } => {
            let address = values.get(address).copied().ok_or_else(|| {
                BackendEmitError(format!(
                    "Cranelift scalar loan address {} is unavailable",
                    address.0
                ))
            })?;
            let load_type = scalar_type(module, *ty)?.ok_or_else(|| {
                BackendEmitError("Cranelift scalar loan load requires a scalar type".to_owned())
            })?;
            let value = builder.ins().load(load_type, MemFlagsData::new(), address, 0);
            values.insert(*result, value);
            Ok(())
        }
        BackendInstruction::ScalarLoanStore { address, value, ty, .. } => {
            let address = values.get(address).copied().ok_or_else(|| {
                BackendEmitError(format!(
                    "Cranelift scalar loan address {} is unavailable",
                    address.0
                ))
            })?;
            let value = values.get(value).copied().ok_or_else(|| {
                BackendEmitError(format!("Cranelift scalar loan value {} is unavailable", value.0))
            })?;
            let store_type = scalar_type(module, *ty)?.ok_or_else(|| {
                BackendEmitError("Cranelift scalar loan store requires a scalar type".to_owned())
            })?;
            if builder.func.dfg.value_type(value) != store_type {
                return Err(BackendEmitError(
                    "Cranelift scalar loan value type mismatch".to_owned(),
                ));
            }
            builder.ins().store(MemFlagsData::new(), value, address, 0);
            Ok(())
        }
        BackendInstruction::ScalarLoanMaterialize { result, value, ty, .. } => {
            let value = values.get(value).copied().ok_or_else(|| {
                BackendEmitError(format!(
                    "Cranelift scalar loan materialization value {} is unavailable",
                    value.0
                ))
            })?;
            let scalar_type = scalar_type(module, *ty)?.ok_or_else(|| {
                BackendEmitError(
                    "Cranelift scalar loan materialization requires a scalar type".to_owned(),
                )
            })?;
            if builder.func.dfg.value_type(value) != scalar_type {
                return Err(BackendEmitError(
                    "Cranelift scalar loan materialization type mismatch".to_owned(),
                ));
            }
            let slot =
                builder.func.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
                    cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                    scalar_type.bytes(),
                    0,
                ));
            let address = builder.ins().stack_addr(module.isa().pointer_type(), slot, 0);
            builder.ins().store(MemFlagsData::new(), value, address, 0);
            values.insert(*result, address);
            Ok(())
        }
        _ => Err(BackendEmitError("Cranelift scalar loan instruction expected".to_owned())),
    }
}

pub(super) fn lower_branch(
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    blocks: &BTreeMap<crate::codegen::BackendBlockId, Block>,
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::Branch {
        condition,
        then_block,
        else_block,
        then_arguments,
        else_arguments,
        ..
    } = instruction
    else {
        return Err(BackendEmitError("Cranelift branch instruction expected".to_owned()));
    };
    let condition = values.get(condition).copied().ok_or_else(|| {
        BackendEmitError(format!("Cranelift branch value {} is unavailable", condition.0))
    })?;
    let condition = builder.ins().icmp_imm_u(IntCC::NotEqual, condition, 0);
    let then_block = blocks
        .get(then_block)
        .copied()
        .ok_or_else(|| BackendEmitError("Cranelift branch true block is unavailable".to_owned()))?;
    let else_block = blocks.get(else_block).copied().ok_or_else(|| {
        BackendEmitError("Cranelift branch false block is unavailable".to_owned())
    })?;
    let then_arguments = edge_arguments(then_arguments, values)?;
    let else_arguments = edge_arguments(else_arguments, values)?;
    builder.ins().brif(condition, then_block, &then_arguments, else_block, &else_arguments);
    Ok(())
}

pub(super) fn lower_jump(
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    blocks: &BTreeMap<crate::codegen::BackendBlockId, Block>,
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::Jump { target, arguments, .. } = instruction else {
        return Err(BackendEmitError("Cranelift jump instruction expected".to_owned()));
    };
    let target = blocks
        .get(target)
        .copied()
        .ok_or_else(|| BackendEmitError("Cranelift jump target block is unavailable".to_owned()))?;
    let arguments = edge_arguments(arguments, values)?;
    builder.ins().jump(target, &arguments);
    Ok(())
}

fn edge_arguments(
    arguments: &[crate::codegen::BackendEdgeArgument],
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<Vec<BlockArg>, BackendEmitError> {
    arguments
        .iter()
        .map(|argument| {
            values.get(&argument.value).copied().map(BlockArg::Value).ok_or_else(|| {
                BackendEmitError(format!(
                    "Cranelift edge argument value {} is unavailable",
                    argument.value.0
                ))
            })
        })
        .collect()
}
