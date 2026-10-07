use std::collections::HashMap;

use cranelift_codegen::ir::{
    InstBuilder, MemFlagsData, StackSlotData, StackSlotKind, TrapCode, Value, condcodes::IntCC,
};
use cranelift_frontend::FunctionBuilder;

use crate::ast::{BinaryOp, Expr};

use super::expressions::{coerce_to_ir_type, lower_expression};
use super::layout::LayoutRegistry;
use super::literals::StringDataValues;
use super::model::NativeCleanupSchedule;
use super::native::{FunctionRef, NativeEmitError};
use super::structs::copy_bytes;
use super::types::NativeType;

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_array_index(
    function: &mut FunctionBuilder<'_>,
    target: &Expr,
    index: &Expr,
    locals: &HashMap<&String, Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Value, NativeEmitError> {
    let (address, element) = lower_array_address(
        function,
        target,
        index,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    if matches!(element, NativeType::Struct(_) | NativeType::Array(_))
        || layouts.is_inline_pack(element)
    {
        return Ok(address);
    }
    let value = function.ins().load(layouts.ir_type(element)?, MemFlagsData::new(), address, 0);
    Ok(swap_indexed_pack_element_if_needed(function, target, value, element, local_types, layouts))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_array_assignment(
    function: &mut FunctionBuilder<'_>,
    target: &Expr,
    index: &Expr,
    value: &Expr,
    locals: &HashMap<&String, Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    let (address, element) = lower_array_address(
        function,
        target,
        index,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    let value = lower_expression(
        function,
        value,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    if matches!(element, NativeType::Struct(_) | NativeType::Array(_))
        || layouts.is_inline_pack(element)
    {
        let size = layouts
            .type_size(element)
            .ok_or_else(|| NativeEmitError("array element has no native size".to_owned()))?;
        copy_bytes(function, value, address, size);
    } else {
        let value = coerce_to_ir_type(function, value, layouts.ir_type(element)?);
        let value = swap_indexed_pack_element_if_needed(
            function,
            target,
            value,
            element,
            local_types,
            layouts,
        );
        function.ins().store(MemFlagsData::new(), value, address, 0);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_array_compound_assignment(
    function: &mut FunctionBuilder<'_>,
    target: &Expr,
    index: &Expr,
    operator: BinaryOp,
    value: &Expr,
    locals: &HashMap<&String, Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    let (address, element) = lower_array_address(
        function,
        target,
        index,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    if matches!(element, NativeType::Struct(_) | NativeType::Array(_))
        || layouts.is_inline_pack(element)
    {
        return Err(NativeEmitError(
            "compound assignment requires a scalar array element".to_owned(),
        ));
    }
    let updated = lower_array_compound_value(
        function,
        target,
        operator,
        address,
        value,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        element,
        layouts,
    )?;
    function.ins().store(MemFlagsData::new(), updated, address, 0);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn lower_array_compound_value(
    function: &mut FunctionBuilder<'_>,
    target: &Expr,
    operator: BinaryOp,
    address: Value,
    value: &Expr,
    locals: &HashMap<&String, Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    element: NativeType,
    layouts: &LayoutRegistry,
) -> Result<Value, NativeEmitError> {
    let current = function.ins().load(layouts.ir_type(element)?, MemFlagsData::new(), address, 0);
    let current = swap_indexed_pack_element_if_needed(
        function,
        target,
        current,
        element,
        local_types,
        layouts,
    );
    let right = lower_expression(
        function,
        value,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    let right = coerce_to_ir_type(function, right, layouts.ir_type(element)?);
    let updated = super::expressions::lower_compound_integer_operation(
        function, current, operator, right, element,
    )?;
    Ok(swap_indexed_pack_element_if_needed(
        function,
        target,
        updated,
        element,
        local_types,
        layouts,
    ))
}

fn swap_indexed_pack_element_if_needed(
    function: &mut FunctionBuilder<'_>,
    target: &Expr,
    value: Value,
    element: NativeType,
    local_types: &HashMap<&String, NativeType>,
    layouts: &LayoutRegistry,
) -> Value {
    if super::structs::indexed_pack_endianness(target, local_types, layouts)
        != Some(crate::ast::LayoutEndianness::Big)
    {
        return value;
    }
    match element {
        NativeType::Integer { width, .. } if width > 8 => function.ins().bswap(value),
        _ => value,
    }
}

pub(super) fn lower_array_constructor(
    function: &mut FunctionBuilder<'_>,
    id: usize,
    layouts: &LayoutRegistry,
) -> Result<Value, NativeEmitError> {
    let layout =
        layouts.array(id).ok_or_else(|| NativeEmitError(format!("missing array layout `{id}")))?;
    let slot = function.func.create_sized_stack_slot(StackSlotData::new(
        StackSlotKind::ExplicitSlot,
        layout.size,
        layout.alignment.trailing_zeros() as u8,
    ));
    let address = function.ins().stack_addr(layouts.pointer_type, slot, 0);
    function.emit_small_memset(
        layouts.frontend_config()?,
        address,
        0u8,
        u64::from(layout.size),
        layout.alignment.trailing_zeros() as u8,
        MemFlagsData::new(),
    );
    Ok(address)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn lower_array_address(
    function: &mut FunctionBuilder<'_>,
    target: &Expr,
    index: &Expr,
    locals: &HashMap<&String, Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<(Value, NativeType), NativeEmitError> {
    let NativeType::Array(id) =
        super::structs::expression_native_type(target, local_types, layouts)
            .ok_or_else(|| NativeEmitError("indexed target has no native array type".to_owned()))?
    else {
        return Err(NativeEmitError("indexed target is not a native array".to_owned()));
    };
    let array =
        layouts.array(id).ok_or_else(|| NativeEmitError("missing array layout".to_owned()))?;
    let base = lower_expression(
        function,
        target,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    let index = lower_expression(
        function,
        index,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    let index = coerce_index(function, index, layouts.pointer_type);
    let capacity = function.ins().iconst(layouts.pointer_type, i64::from(array.capacity));
    let in_bounds = function.ins().icmp(IntCC::UnsignedLessThan, index, capacity);
    let ok_block = function.create_block();
    let trap_block = function.create_block();
    function.ins().brif(in_bounds, ok_block, &[], trap_block, &[]);
    function.switch_to_block(trap_block);
    function.ins().trap(TrapCode::HEAP_OUT_OF_BOUNDS);
    function.seal_block(trap_block);
    function.switch_to_block(ok_block);
    let element_size = layouts
        .type_size(array.element)
        .ok_or_else(|| NativeEmitError("array element has no native size".to_owned()))?;
    let byte_offset = function.ins().imul_imm_u(index, i64::from(element_size));
    let address = function.ins().iadd(base, byte_offset);
    function.seal_block(ok_block);
    Ok((address, array.element))
}

fn coerce_index(
    function: &mut FunctionBuilder<'_>,
    index: Value,
    pointer_type: cranelift_codegen::ir::Type,
) -> Value {
    let source = function.func.dfg.value_type(index);
    if source == pointer_type {
        return index;
    }
    if source.bytes() < pointer_type.bytes() {
        function.ins().uextend(pointer_type, index)
    } else {
        function.ins().ireduce(pointer_type, index)
    }
}
