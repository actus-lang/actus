use std::collections::HashMap;

use cranelift_codegen::ir::Value;
use cranelift_frontend::FunctionBuilder;

use super::super::native::NativeEmitError;
use super::super::types::NativeType;

pub(super) fn seal_checked_pack_block(
    function: &mut FunctionBuilder<'_>,
    field_type: NativeType,
    block_name: &str,
) {
    if field_type == (NativeType::Integer { signed: false, width: 8 }) {
        let block = function.current_block().expect(block_name);
        function.seal_block(block);
    }
}

pub(super) fn update_pack_binding(
    locals: &mut HashMap<&String, Value>,
    name: &str,
    updated: Value,
) -> Result<(), NativeEmitError> {
    let binding = locals
        .keys()
        .find(|candidate| candidate.as_str() == name)
        .copied()
        .ok_or_else(|| NativeEmitError(format!("native binding `{name}` is unavailable")))?;
    locals.insert(binding, updated);
    Ok(())
}
