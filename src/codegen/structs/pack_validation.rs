use cranelift_codegen::ir::{InstBuilder, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;

use super::super::expressions::coerce_to_ir_type;
use super::super::native::NativeEmitError;
use super::super::types::NativeType;

pub(super) fn validate_unsigned_byte_value(
    function: &mut FunctionBuilder<'_>,
    value: Value,
    field_type: NativeType,
) -> Result<(), NativeEmitError> {
    if field_type != (NativeType::Integer { signed: false, width: 8 }) {
        return Ok(());
    }
    let source_type = function.func.dfg.value_type(value);
    let comparison_type =
        if source_type.bytes() < types::I32.bytes() { types::I32 } else { source_type };
    let value = coerce_to_ir_type(function, value, comparison_type);
    let limit = function.ins().iconst(comparison_type, 256);
    let valid = function.ins().icmp(IntCC::UnsignedLessThan, value, limit);
    let ok_block = function.create_block();
    let trap_block = function.create_block();
    function.ins().brif(valid, ok_block, &[], trap_block, &[]);
    function.switch_to_block(trap_block);
    function.ins().trap(cranelift_codegen::ir::TrapCode::HEAP_OUT_OF_BOUNDS);
    function.seal_block(trap_block);
    function.switch_to_block(ok_block);
    Ok(())
}
