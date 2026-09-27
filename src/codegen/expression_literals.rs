use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, Type, types};
use cranelift_frontend::FunctionBuilder;

use super::literals::StringDataValues;
use super::native::NativeEmitError;

pub(super) fn lower_integer(
    function: &mut FunctionBuilder<'_>,
    value: &str,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    value
        .parse::<i32>()
        .map(|value| function.ins().iconst(types::I32, i64::from(value)))
        .map_err(|error| NativeEmitError(format!("invalid integer literal: {error}")))
}

pub(super) fn lower_float(
    function: &mut FunctionBuilder<'_>,
    value: &str,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    value
        .parse::<f64>()
        .map(|value| function.ins().f64const(value))
        .map_err(|error| NativeEmitError(format!("invalid floating-point literal: {error}")))
}

pub(super) fn lower_float_as(
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

pub(super) fn coerce_to_ir_type(
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

pub(super) fn lower_string(
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

pub(super) fn lower_identifier(
    name: &str,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    locals
        .iter()
        .find(|(binding, _)| binding.as_str() == name)
        .map(|(_, value)| *value)
        .ok_or_else(|| NativeEmitError(format!("native binding `{name}` is unavailable")))
}
