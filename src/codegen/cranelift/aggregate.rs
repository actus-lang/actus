use std::collections::BTreeMap;

use cranelift_codegen::ir::{Block, Type};
use cranelift_frontend::FunctionBuilder;
use cranelift_object::ObjectModule;

use super::native::BackendEmitError;
use crate::codegen::{BackendFunction, BackendPassingMode, BackendValueId};

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
