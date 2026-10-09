use cranelift_codegen::ir::{InstBuilder, StackSlotData, StackSlotKind, Value};
use cranelift_frontend::FunctionBuilder;

use super::super::layout::LayoutRegistry;
use super::super::native::NativeEmitError;
use super::super::types::NativeType;

pub(super) fn allocate_return_address(
    function: &mut FunctionBuilder<'_>,
    return_type: NativeType,
    layouts: &LayoutRegistry,
) -> Result<Option<Value>, NativeEmitError> {
    if !layouts.uses_return_slot(return_type) {
        return Ok(None);
    }
    let slot = match return_type {
        NativeType::Struct(id) => {
            let layout = layouts
                .get(id)
                .ok_or_else(|| NativeEmitError(format!("missing return layout `{id}`")))?;
            function.func.create_sized_stack_slot(layouts.stack_slot(layout))
        }
        _ => create_scalar_return_slot(function, return_type, layouts)?,
    };
    Ok(Some(function.ins().stack_addr(layouts.pointer_type, slot, 0)))
}

fn create_scalar_return_slot(
    function: &mut FunctionBuilder<'_>,
    return_type: NativeType,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::StackSlot, NativeEmitError> {
    let size = layouts
        .return_slot_size(return_type)
        .ok_or_else(|| NativeEmitError("missing sret return layout".to_owned()))?;
    Ok(function.func.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        size,
        4,
    )))
}
