// Aggregate emission returns the native value maps needed by later lowering.
#![allow(clippy::type_complexity)]

use std::collections::BTreeMap;

use cranelift_codegen::ir::{Block, InstBuilder, Type};
use cranelift_frontend::FunctionBuilder;
use cranelift_module::Module;
use cranelift_object::ObjectModule;

use super::native::BackendEmitError;
use crate::codegen::{
    BackendFunction, BackendInstruction, BackendPassingMode, BackendValueId, BackendValueType,
};

#[derive(Clone, Debug)]
pub(super) struct Declaration {
    pub(super) id: cranelift_module::FuncId,
    pub(super) parameter_fragments: Vec<usize>,
    pub(super) parameter_types: Vec<Vec<Type>>,
    pub(super) return_fragments: usize,
    pub(super) return_slot: Option<crate::codegen::BackendLayout>,
    pub(super) runtime_pointer_return: bool,
    pub(super) niche_pointer_return: bool,
    pub(super) runtime_result_layout: Option<crate::codegen::BackendLayout>,
}

pub(super) fn declaration(
    module: &ObjectModule,
    id: cranelift_module::FuncId,
    parameters: &[crate::codegen::BackendParameter],
    return_value: &crate::codegen::BackendReturn,
) -> Result<Declaration, BackendEmitError> {
    let return_slot = match &return_value.passing {
        crate::codegen::BackendReturnMode::ReturnSlot { layout } => {
            layout.validate().map_err(|error| BackendEmitError(error.to_string()))?;
            Some(*layout)
        }
        _ => None,
    };
    Ok(Declaration {
        id,
        parameter_fragments: parameters
            .iter()
            .map(parameter_fragment_count)
            .collect::<Result<_, _>>()?,
        parameter_types: parameters
            .iter()
            .map(|parameter| super::contract::parameter_types(module, parameter))
            .collect::<Result<_, _>>()?,
        return_fragments: return_fragment_count(return_value, false)?,
        return_slot,
        runtime_pointer_return: false,
        niche_pointer_return: matches!(
            return_value.ty,
            BackendValueType::NichePointerOption { .. }
        ),
        runtime_result_layout: None,
    })
}

pub(super) fn runtime_declaration(
    module: &ObjectModule,
    id: cranelift_module::FuncId,
    external: &crate::codegen::BackendExternal,
) -> Result<Declaration, BackendEmitError> {
    let mut declaration = declaration(module, id, &external.parameters, &external.return_value)?;
    declaration.runtime_pointer_return = external.runtime_pointer_return;
    if external.runtime_pointer_return {
        let caller_owned_return_slot =
            matches!(external.return_value.ty, BackendValueType::NichePointerOption { .. });
        if !caller_owned_return_slot {
            declaration.return_slot = None;
        }
        declaration.return_fragments = 1;
        declaration.runtime_result_layout = match external.return_value.passing {
            crate::codegen::BackendReturnMode::ReturnSlot { layout } => Some(layout),
            _ => None,
        };
    }
    Ok(declaration)
}

fn parameter_fragment_count(
    parameter: &crate::codegen::BackendParameter,
) -> Result<usize, BackendEmitError> {
    match &parameter.passing {
        BackendPassingMode::DirectAggregate { abi } => {
            abi.validate().map_err(|error| BackendEmitError(error.to_string()))?;
            Ok(abi.parameter_fragments.len())
        }
        _ => Ok(1),
    }
}

fn return_fragment_count(
    return_value: &crate::codegen::BackendReturn,
    runtime_pointer_return: bool,
) -> Result<usize, BackendEmitError> {
    match &return_value.passing {
        crate::codegen::BackendReturnMode::DirectAggregate { abi } => {
            abi.validate().map_err(|error| BackendEmitError(error.to_string()))?;
            Ok(abi.return_fragments.len())
        }
        crate::codegen::BackendReturnMode::Direct => {
            Ok(usize::from(!matches!(return_value.ty, BackendValueType::Unit)))
        }
        crate::codegen::BackendReturnMode::ReturnSlot { .. } => {
            Ok(usize::from(runtime_pointer_return))
        }
    }
}

pub(super) fn lower_call(
    module: &mut ObjectModule,
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    declarations: &BTreeMap<String, Declaration>,
    function_return_slot: Option<cranelift_codegen::ir::Value>,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
    aggregate_values: &mut BTreeMap<BackendValueId, Vec<cranelift_codegen::ir::Value>>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::Call { callee, arguments, result, .. } = instruction else {
        return Err(BackendEmitError("Cranelift call instruction expected".to_owned()));
    };
    let declaration = declarations.get(callee.as_str()).ok_or_else(|| {
        BackendEmitError(format!("Cranelift call `{}` is undeclared", callee.as_str()))
    })?;
    let reference = module.declare_func_in_func(declaration.id, builder.func);
    let native_arguments = collect_call_arguments(
        builder,
        callee.as_str(),
        arguments,
        declaration,
        values,
        aggregate_values,
    )?;
    let (return_slot, native_arguments) = if declaration.niche_pointer_return
        && let Some(address) = function_return_slot
    {
        let mut arguments = Vec::with_capacity(native_arguments.len() + 1);
        arguments.push(address);
        arguments.extend(native_arguments);
        (Some(address), arguments)
    } else if let Some(layout) = declaration.return_slot {
        let slot = create_return_slot(builder, layout)?;
        let address = builder.ins().stack_addr(module.isa().pointer_type(), slot, 0);
        (Some(address), {
            let mut arguments = Vec::with_capacity(native_arguments.len() + 1);
            arguments.push(address);
            arguments.extend(native_arguments);
            arguments
        })
    } else {
        (None, native_arguments)
    };
    let call = builder.ins().call(reference, &native_arguments);
    if declaration.runtime_pointer_return {
        let result = result.ok_or_else(|| {
            BackendEmitError("Cranelift runtime-pointer call requires a result value".to_owned())
        })?;
        let raw = builder.inst_results(call).first().copied().ok_or_else(|| {
            BackendEmitError("Cranelift runtime result is unavailable".to_owned())
        })?;
        if declaration.niche_pointer_return {
            let return_slot = return_slot.ok_or_else(|| {
                BackendEmitError("Cranelift niche runtime return slot is unavailable".to_owned())
            })?;
            values.insert(result, return_slot);
            return Ok(());
        }
        let layout = declaration.runtime_result_layout.ok_or_else(|| {
            BackendEmitError("Cranelift runtime-pointer result layout is unavailable".to_owned())
        })?;
        let slot = create_return_slot(builder, layout)?;
        let address = builder.ins().stack_addr(module.isa().pointer_type(), slot, 0);
        let alignment = u8::try_from(layout.alignment).map_err(|_| {
            BackendEmitError("Cranelift runtime result alignment exceeds u8".to_owned())
        })?;
        builder.emit_small_memory_copy(
            module.isa().frontend_config(),
            address,
            raw,
            u64::from(layout.size),
            alignment,
            alignment,
            false,
            cranelift_codegen::ir::MemFlagsData::new(),
        );
        values.insert(result, address);
        return Ok(());
    }
    record_call_results(builder, call, result, return_slot, declaration, values, aggregate_values)?;
    Ok(())
}

fn create_return_slot(
    builder: &mut FunctionBuilder<'_>,
    layout: crate::codegen::BackendLayout,
) -> Result<cranelift_codegen::ir::StackSlot, BackendEmitError> {
    let alignment = u8::try_from(layout.alignment)
        .map_err(|_| BackendEmitError("Cranelift return slot alignment exceeds u8".to_owned()))?;
    let align_shift = alignment.trailing_zeros() as u8;
    Ok(builder.func.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
        cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
        layout.size,
        align_shift,
    )))
}

fn collect_call_arguments(
    builder: &mut FunctionBuilder<'_>,
    callee: &str,
    arguments: &[BackendValueId],
    declaration: &Declaration,
    values: &BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
    aggregate_values: &BTreeMap<BackendValueId, Vec<cranelift_codegen::ir::Value>>,
) -> Result<Vec<cranelift_codegen::ir::Value>, BackendEmitError> {
    if declaration.parameter_fragments.len() != arguments.len() {
        return Err(BackendEmitError(format!(
            "Cranelift call `{callee}` expects {} arguments, got {}",
            declaration.parameter_fragments.len(),
            arguments.len()
        )));
    }
    let mut native_arguments = Vec::new();
    for ((value, fragment_count), expected_types) in
        arguments.iter().zip(&declaration.parameter_fragments).zip(&declaration.parameter_types)
    {
        if *fragment_count > 1 {
            let fragments = aggregate_values.get(value).ok_or_else(|| {
                BackendEmitError(format!(
                    "Cranelift aggregate call value {} is unavailable",
                    value.0
                ))
            })?;
            if fragments.len() != *fragment_count {
                return Err(BackendEmitError(
                    "Cranelift aggregate call fragment count mismatches declaration".to_owned(),
                ));
            }
            native_arguments.extend(fragments.iter().copied());
        } else if let Some(value) = values.get(value).copied() {
            native_arguments.push(coerce_call_value(
                builder,
                value,
                expected_types.first().copied(),
            ));
        } else {
            return Err(BackendEmitError(format!(
                "Cranelift call value {} is unavailable",
                value.0
            )));
        }
    }
    Ok(native_arguments)
}

fn coerce_call_value(
    builder: &mut FunctionBuilder<'_>,
    value: cranelift_codegen::ir::Value,
    expected: Option<Type>,
) -> cranelift_codegen::ir::Value {
    let Some(expected) = expected else { return value };
    let actual = builder.func.dfg.value_type(value);
    if actual == expected {
        return value;
    }
    if actual.is_int() && expected.is_int() {
        if actual.bytes() < expected.bytes() {
            return builder.ins().uextend(expected, value);
        }
        return builder.ins().ireduce(expected, value);
    }
    value
}

fn record_call_results(
    builder: &mut FunctionBuilder<'_>,
    call: cranelift_codegen::ir::Inst,
    result: &Option<BackendValueId>,
    return_slot: Option<cranelift_codegen::ir::Value>,
    declaration: &Declaration,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
    aggregate_values: &mut BTreeMap<BackendValueId, Vec<cranelift_codegen::ir::Value>>,
) -> Result<(), BackendEmitError> {
    let Some(result) = result else {
        if return_slot.is_some() {
            return Err(BackendEmitError(
                "Cranelift return-slot call requires a result value".to_owned(),
            ));
        }
        return Ok(());
    };
    if let Some(return_slot) = return_slot {
        values.insert(*result, return_slot);
        return Ok(());
    }
    let results = builder.inst_results(call).to_vec();
    if declaration.return_fragments > 1 {
        if results.len() != declaration.return_fragments {
            return Err(BackendEmitError(
                "Cranelift aggregate call result count mismatches declaration".to_owned(),
            ));
        }
        aggregate_values.insert(*result, results);
    } else if declaration.return_fragments == 1 {
        let value = results
            .first()
            .copied()
            .ok_or_else(|| BackendEmitError("Cranelift call result is unavailable".to_owned()))?;
        values.insert(*result, value);
    } else {
        return Err(BackendEmitError("Cranelift unit call cannot produce a result".to_owned()));
    }
    Ok(())
}

pub(super) fn direct_aggregate_types(
    _module: &ObjectModule,
    abi: &crate::codegen::BackendDirectAggregateAbi,
    parameters: bool,
) -> Result<Vec<Type>, BackendEmitError> {
    abi.validate().map_err(|error| BackendEmitError(error.to_string()))?;
    let fragments = if parameters { &abi.parameter_fragments } else { &abi.return_fragments };
    let mut expected_register = 0u16;
    let mut types = Vec::with_capacity(fragments.len());
    for fragment in fragments {
        let crate::codegen::BackendAggregateLocation::Register { index } = fragment.location else {
            return Err(BackendEmitError(
                "Cranelift direct aggregate ABI currently requires register fragments".to_owned(),
            ));
        };
        if index != expected_register {
            return Err(BackendEmitError(
                "Cranelift direct aggregate register fragments must be contiguous".to_owned(),
            ));
        }
        expected_register = expected_register.saturating_add(1);
        let size = u8::try_from(fragment.size).map_err(|_| {
            BackendEmitError("Cranelift aggregate fragment exceeds integer width".to_owned())
        })?;
        types.push(aggregate_fragment_type(size)?);
    }
    Ok(types)
}

pub(super) fn aggregate_fragment_type(size: u8) -> Result<Type, BackendEmitError> {
    Type::int(u16::from(size) * 8)
        .ok_or_else(|| BackendEmitError("Cranelift integer width is invalid".to_owned()))
}

pub(super) fn initialize_values(
    builder: &mut FunctionBuilder<'_>,
    entry: Block,
    function: &BackendFunction,
) -> Result<
    (
        Option<cranelift_codegen::ir::Value>,
        BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
        BTreeMap<BackendValueId, Vec<cranelift_codegen::ir::Value>>,
    ),
    BackendEmitError,
> {
    builder.append_block_params_for_function_params(entry);
    let block_params = builder.block_params(entry).to_vec();
    let return_slot = matches!(
        &function.return_value.passing,
        crate::codegen::BackendReturnMode::ReturnSlot { .. }
    )
    .then(|| block_params.first().copied())
    .flatten();
    let mut physical_index = usize::from(return_slot.is_some());
    let mut values = BTreeMap::new();
    let mut aggregate_values = BTreeMap::new();
    for (index, parameter) in function.parameters.iter().enumerate() {
        let value_id = BackendValueId(index as u32);
        if let BackendPassingMode::DirectAggregate { abi } = &parameter.passing {
            let count = abi.parameter_fragments.len();
            let end = physical_index.saturating_add(count);
            let fragments = block_params.get(physical_index..end).ok_or_else(|| {
                BackendEmitError(
                    "Cranelift aggregate parameter mapping exceeds signature".to_owned(),
                )
            })?;
            aggregate_values.insert(value_id, fragments.to_vec());
            physical_index = end;
        } else if let Some(value) = block_params.get(physical_index).copied() {
            values.insert(value_id, value);
            physical_index += 1;
        } else {
            return Err(BackendEmitError(
                "Cranelift parameter mapping exceeds signature".to_owned(),
            ));
        }
    }
    Ok((return_slot, values, aggregate_values))
}
