use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData, StackSlotData, StackSlotKind, Type, types};
use cranelift_frontend::FunctionBuilder;

use super::super::literals::StringDataValues;
use super::super::native::NativeEmitError;

pub(in crate::codegen) fn lower_integer(
    function: &mut FunctionBuilder<'_>,
    value: &str,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let is_hex = value.starts_with("0x") || value.starts_with("0X");
    let magnitude = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .map_or_else(|| value.parse::<u128>(), |digits| u128::from_str_radix(digits, 16))
        .map_err(|error| NativeEmitError(format!("invalid integer literal: {error}")))?;
    if magnitude <= i64::MAX as u128 {
        if !is_hex && magnitude > i32::MAX as u128 {
            return Err(NativeEmitError(
                "invalid integer literal: exceeds native Int width".to_owned(),
            ));
        }
        let target = if magnitude <= i32::MAX as u128 { types::I32 } else { types::I64 };
        return Ok(function.ins().iconst(target, magnitude as i64));
    }
    Err(NativeEmitError("integer literal exceeds native lowering width".to_owned()))
}

pub(in crate::codegen) fn lower_float(
    function: &mut FunctionBuilder<'_>,
    value: &str,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    value
        .parse::<f64>()
        .map(|value| function.ins().f64const(value))
        .map_err(|error| NativeEmitError(format!("invalid floating-point literal: {error}")))
}

pub(in crate::codegen) fn lower_wide_integer(
    function: &mut FunctionBuilder<'_>,
    value: &str,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let magnitude = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .map_or_else(|| value.parse::<u128>(), |digits| u128::from_str_radix(digits, 16))
        .map_err(|error| NativeEmitError(format!("invalid wide integer literal: {error}")))?;
    let slot = function.func.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        16,
        4,
    ));
    let address = function.ins().stack_addr(types::I64, slot, 0);
    let low = function.ins().iconst(types::I64, magnitude as i64);
    let high = function.ins().iconst(types::I64, (magnitude >> 64) as i64);
    function.ins().store(MemFlagsData::new(), low, address, 0);
    function.ins().store(MemFlagsData::new(), high, address, 8);
    Ok(address)
}

pub(in crate::codegen) fn lower_float_as(
    function: &mut FunctionBuilder<'_>,
    value: &str,
    target: Type,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let value = value
        .parse::<f64>()
        .map_err(|error| NativeEmitError(format!("invalid floating-point literal: {error}")))?;
    Ok(if target == types::F32 {
        function.ins().f32const(value as f32)
    } else {
        function.ins().f64const(value)
    })
}

pub(in crate::codegen) fn coerce_to_ir_type(
    function: &mut FunctionBuilder<'_>,
    value: cranelift_codegen::ir::Value,
    target: Type,
) -> cranelift_codegen::ir::Value {
    let source = function.func.dfg.value_type(value);
    if source == target {
        return value;
    }
    if source.is_int() && target.is_int() {
        return if source.bytes() > target.bytes() {
            function.ins().ireduce(target, value)
        } else {
            function.ins().uextend(target, value)
        };
    }
    value
}

pub(in crate::codegen) fn lower_string(
    function: &mut FunctionBuilder<'_>,
    value: &str,
    string_data: &StringDataValues,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    string_data
        .get(value)
        .copied()
        .map(|global| function.ins().symbol_value(types::I64, global))
        .ok_or_else(|| NativeEmitError(format!("string literal `{value}` has no native data")))
}

pub(in crate::codegen) fn lower_identifier(
    name: &str,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    locals
        .iter()
        .find(|(binding, _)| binding.as_str() == name)
        .map(|(_, value)| *value)
        .ok_or_else(|| NativeEmitError(format!("native binding `{name}` is unavailable")))
}
