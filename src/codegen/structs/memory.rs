use cranelift_codegen::ir::{InstBuilder, MemFlagsData};
use cranelift_frontend::FunctionBuilder;

use super::super::layout::{FieldLayout, LayoutRegistry};
use super::super::native::NativeEmitError;
use super::super::types::NativeType;

pub(super) fn store_struct_field(
    function: &mut FunctionBuilder<'_>,
    address: cranelift_codegen::ir::Value,
    value: cranelift_codegen::ir::Value,
    field: &FieldLayout,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    if field.indirect || matches!(field.ty, NativeType::Enum(id) if layouts.is_niche_option(id)) {
        function.ins().store(MemFlagsData::new(), value, address, field.offset as i32);
    } else if matches!(field.ty, NativeType::Struct(_) | NativeType::Enum(_) | NativeType::Array(_))
        || layouts.is_inline_pack(field.ty)
    {
        let size = layouts
            .type_size(field.ty)
            .ok_or_else(|| NativeEmitError("missing nested field layout".to_owned()))?;
        let destination = function.ins().iadd_imm_s(address, i64::from(field.offset));
        copy_bytes(function, value, destination, size);
    } else {
        function.ins().store(MemFlagsData::new(), value, address, field.offset as i32);
    }
    Ok(())
}

pub(crate) fn copy_bytes(
    function: &mut FunctionBuilder<'_>,
    source: cranelift_codegen::ir::Value,
    destination: cranelift_codegen::ir::Value,
    size: u32,
) {
    for offset in 0..size {
        let source_address = function.ins().iadd_imm_s(source, i64::from(offset));
        let destination_address = function.ins().iadd_imm_s(destination, i64::from(offset));
        let byte = function.ins().load(
            cranelift_codegen::ir::types::I8,
            MemFlagsData::new(),
            source_address,
            0,
        );
        function.ins().store(MemFlagsData::new(), byte, destination_address, 0);
    }
}
