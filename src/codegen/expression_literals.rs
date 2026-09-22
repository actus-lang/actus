use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, types};
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
