use std::collections::BTreeMap;

mod cleanup;
pub(super) use cleanup::LayoutCleanupProvider;

use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{AbiParam, Block, InstBuilder, MemFlagsData, TrapCode, Type, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module};
use cranelift_object::ObjectModule;

use super::aggregate::{
    Declaration, aggregate_fragment_type, declaration, direct_aggregate_types, initialize_values,
    lower_call,
};
use super::integer_ops::{
    lower_integer_binary, lower_integer_compare, lower_integer_unary,
    lower_trap_if_shift_count_out_of_range, lower_trap_if_signed_division_overflow,
    lower_trap_if_zero,
};
use super::native::BackendEmitError;
use super::object::create_module;
use crate::codegen::{
    BackendFunction, BackendInstruction, BackendModule, BackendPassingMode, BackendValueId,
    BackendValueType,
};
use crate::configuration::NativeBackendConfiguration;
use crate::target::TargetSpec;

pub fn emit_contract_module_object(
    backend_module: &BackendModule,
    configuration: &NativeBackendConfiguration,
    target: &TargetSpec,
) -> Result<Vec<u8>, BackendEmitError> {
    backend_module.validate().map_err(|error| BackendEmitError(error.to_string()))?;
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
        let signature = signature(module, &external.parameters, &external.return_value)?;
        let id = module
            .declare_function(external.symbol.as_str(), Linkage::Import, &signature)
            .map_err(|error| BackendEmitError(error.to_string()))?;
        declarations.insert(
            external.symbol.as_str().to_owned(),
            declaration(id, &external.parameters, &external.return_value)?,
        );
    }
    for function in &backend_module.functions {
        let signature = signature(module, &function.parameters, &function.return_value)?;
        let id = module
            .declare_function(function.symbol.as_str(), Linkage::Export, &signature)
            .map_err(|error| BackendEmitError(error.to_string()))?;
        declarations.insert(
            function.symbol.as_str().to_owned(),
            declaration(id, &function.parameters, &function.return_value)?,
        );
    }
    cleanup::declare_cleanup_bridges(module, backend_module, &mut declarations)?;
    Ok(declarations)
}

fn signature(
    module: &ObjectModule,
    parameters: &[crate::codegen::BackendParameter],
    return_value: &crate::codegen::BackendReturn,
) -> Result<cranelift_codegen::ir::Signature, BackendEmitError> {
    let mut signature = module.make_signature();
    for parameter in parameters {
        for ty in parameter_types(module, parameter)? {
            signature.params.push(AbiParam::new(ty));
        }
    }
    match &return_value.passing {
        crate::codegen::BackendReturnMode::Direct => {
            if let Some(ty) = scalar_type(module, return_value.ty)? {
                signature.returns.push(AbiParam::new(ty));
            }
        }
        crate::codegen::BackendReturnMode::DirectAggregate { abi } => {
            for ty in direct_aggregate_types(module, abi, false)? {
                signature.returns.push(AbiParam::new(ty));
            }
        }
        crate::codegen::BackendReturnMode::ReturnSlot { layout } => {
            if !matches!(return_value.ty, BackendValueType::Aggregate { layout: value_layout } | BackendValueType::Region { layout: value_layout } if value_layout == *layout)
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

fn parameter_types(
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
                BackendValueType::Aggregate { layout: value_layout }
                | BackendValueType::Region { layout: value_layout }
                    if value_layout == *layout =>
                {
                    Ok(vec![module.isa().pointer_type()])
                }
                BackendValueType::Aggregate { .. }
                | BackendValueType::Region { .. }
                | BackendValueType::NichePointerOption { .. } => Err(BackendEmitError(
                    "Cranelift aggregate parameter layout does not match passing layout".to_owned(),
                )),
                _ => Err(BackendEmitError(
                    "Cranelift indirect parameter must have an aggregate or Region type".to_owned(),
                )),
            }
        }
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
        BackendValueType::NichePointerOption { .. } => Some(module.isa().pointer_type()),
        BackendValueType::Integer { .. }
        | BackendValueType::Float { .. }
        | BackendValueType::Aggregate { .. }
        | BackendValueType::Region { .. } => {
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
    context.func.signature = signature(module, &function.parameters, &function.return_value)?;
    let frontend_config = module.isa().frontend_config();
    let mut function_context = FunctionBuilderContext::new();
    {
        let mut builder = FunctionBuilder::new(&mut context.func, &mut function_context);
        let blocks = create_blocks(&mut builder, function);
        let entry = blocks.get(&function.entry).copied().ok_or_else(|| {
            BackendEmitError("Cranelift neutral contract entry block is missing".to_owned())
        })?;
        builder.switch_to_block(entry);
        let (return_slot, mut values, mut aggregate_values) =
            initialize_values(&mut builder, entry, function)?;
        for block in &function.blocks {
            let native_block = blocks.get(&block.id).copied().ok_or_else(|| {
                BackendEmitError("Cranelift neutral contract block is missing".to_owned())
            })?;
            builder.switch_to_block(native_block);
            for instruction in &block.instructions {
                lower_instruction(
                    module,
                    &mut builder,
                    instruction,
                    declarations,
                    &blocks,
                    return_slot,
                    &mut values,
                    &mut aggregate_values,
                    function,
                )?;
            }
        }
        builder.seal_all_blocks();
        builder.finalize(frontend_config);
    }
    module
        .define_function(declaration.id, &mut context)
        .map_err(|error| BackendEmitError(error.to_string()))?;
    module.clear_context(&mut context);
    Ok(())
}

fn create_blocks(
    builder: &mut FunctionBuilder<'_>,
    function: &BackendFunction,
) -> BTreeMap<crate::codegen::BackendBlockId, Block> {
    function.blocks.iter().map(|block| (block.id, builder.create_block())).collect()
}

fn lower_instruction(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    declarations: &BTreeMap<String, Declaration>,
    blocks: &BTreeMap<crate::codegen::BackendBlockId, Block>,
    return_slot: Option<cranelift_codegen::ir::Value>,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
    aggregate_values: &mut BTreeMap<BackendValueId, Vec<cranelift_codegen::ir::Value>>,
    function: &BackendFunction,
) -> Result<(), BackendEmitError> {
    match instruction {
        BackendInstruction::Constant { .. } => lower_constant(module, builder, instruction, values),
        BackendInstruction::IntegerBinary { .. } => {
            lower_integer_binary(builder, instruction, values)
        }
        BackendInstruction::TrapIfZero { .. }
        | BackendInstruction::TrapIfSignedDivisionOverflow { .. }
        | BackendInstruction::TrapIfShiftCountOutOfRange { .. } => {
            lower_guard(builder, instruction, values)
        }
        BackendInstruction::IntegerCompare { .. } => {
            lower_integer_compare(builder, instruction, values)
        }
        BackendInstruction::IntegerUnary { .. } => {
            lower_integer_unary(builder, instruction, values)
        }
        BackendInstruction::Call { .. } => {
            lower_call(module, builder, instruction, declarations, values, aggregate_values)
        }
        BackendInstruction::Branch { .. } => lower_branch(builder, instruction, blocks, values),
        BackendInstruction::Jump { .. } => lower_jump(builder, instruction, blocks),
        BackendInstruction::Return { .. } => {
            lower_return(builder, instruction, function, values, aggregate_values)
        }
        BackendInstruction::Fail { .. } => {
            lower_failure(module.isa().pointer_type(), builder, instruction, return_slot, values)
        }
        BackendInstruction::MemoryCopy { .. } => {
            lower_memory_copy(module, builder, instruction, values)
        }
        BackendInstruction::Trap { .. } => {
            builder.ins().trap(TrapCode::unwrap_user(1));
            Ok(())
        }
        BackendInstruction::Cleanup { action, operand, .. } => {
            cleanup::lower_cleanup(module, builder, action, *operand, declarations, values)
        }
    }
}

fn lower_guard(
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
        _ => Err(BackendEmitError("Cranelift guard instruction is not supported".to_owned())),
    }
}

fn lower_constant(
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

fn lower_memory_copy(
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

fn lower_branch(
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    blocks: &BTreeMap<crate::codegen::BackendBlockId, Block>,
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::Branch { condition, then_block, else_block, .. } = instruction else {
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
    builder.ins().brif(condition, then_block, &[], else_block, &[]);
    Ok(())
}

fn lower_jump(
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    blocks: &BTreeMap<crate::codegen::BackendBlockId, Block>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::Jump { target, .. } = instruction else {
        return Err(BackendEmitError("Cranelift jump instruction expected".to_owned()));
    };
    let target = blocks
        .get(target)
        .copied()
        .ok_or_else(|| BackendEmitError("Cranelift jump target block is unavailable".to_owned()))?;
    builder.ins().jump(target, &[]);
    Ok(())
}

fn lower_failure(
    pointer_type: Type,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    return_slot: Option<cranelift_codegen::ir::Value>,
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::Fail { failure, .. } = instruction else {
        return Err(BackendEmitError("Cranelift failure instruction expected".to_owned()));
    };
    if failure.layout.niche_pointer {
        return lower_niche_failure(pointer_type, builder, failure, return_slot, values);
    }
    let slot = return_slot.ok_or_else(|| {
        BackendEmitError("Cranelift typed failure requires a return slot".to_owned())
    })?;
    let discriminant_type = aggregate_fragment_type(failure.layout.discriminant_size)?;
    let discriminant = builder.ins().iconst(discriminant_type, i64::from(failure.discriminant));
    builder.ins().store(
        MemFlagsData::new(),
        discriminant,
        slot,
        failure.layout.discriminant_offset as i32,
    );
    if let (Some(payload_id), Some(payload_layout)) = (failure.payload, failure.layout.payload) {
        let payload = values.get(&payload_id).copied().ok_or_else(|| {
            BackendEmitError(format!("Cranelift failure payload {} is unavailable", payload_id.0))
        })?;
        if !builder.func.dfg.value_type(payload).is_int()
            || builder.func.dfg.value_type(payload).bytes() > payload_layout.size
        {
            return Err(BackendEmitError(
                "Cranelift typed failure currently requires a fitting integer payload".to_owned(),
            ));
        }
        builder.ins().store(
            MemFlagsData::new(),
            payload,
            slot,
            failure.layout.payload_offset as i32,
        );
    }
    builder.ins().return_(&[]);
    Ok(())
}

fn lower_niche_failure(
    pointer_type: Type,
    builder: &mut FunctionBuilder<'_>,
    failure: &crate::codegen::BackendFailure,
    return_slot: Option<cranelift_codegen::ir::Value>,
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    if return_slot.is_some() {
        return Err(BackendEmitError(
            "Cranelift niche-pointer failure cannot use an aggregate return slot".to_owned(),
        ));
    }
    let value = failure
        .payload
        .and_then(|payload_id| values.get(&payload_id).copied())
        .unwrap_or_else(|| builder.ins().iconst(pointer_type, 0));
    if builder.func.dfg.value_type(value) != pointer_type {
        return Err(BackendEmitError(
            "Cranelift niche-pointer failure payload must be a pointer".to_owned(),
        ));
    }
    builder.ins().return_(&[value]);
    Ok(())
}

fn lower_return(
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    function: &BackendFunction,
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
    aggregate_values: &BTreeMap<BackendValueId, Vec<cranelift_codegen::ir::Value>>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::Return { value, .. } = instruction else {
        return Err(BackendEmitError("Cranelift return instruction expected".to_owned()));
    };
    if let Some(value_id) = value
        && matches!(
            function.return_value.passing,
            crate::codegen::BackendReturnMode::DirectAggregate { .. }
        )
    {
        let fragments = aggregate_values.get(&value_id).ok_or_else(|| {
            BackendEmitError(format!(
                "Cranelift aggregate return value {} is unavailable",
                value_id.0
            ))
        })?;
        builder.ins().return_(fragments);
        return Ok(());
    }
    let value = value
        .map(|value| {
            values.get(&value).copied().ok_or_else(|| {
                BackendEmitError(format!("Cranelift return value {} is unavailable", value.0))
            })
        })
        .transpose()?;
    match value {
        Some(value) => builder.ins().return_(&[value]),
        None => builder.ins().return_(&[]),
    };
    Ok(())
}
