use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData, TrapCode, condcodes::IntCC};
use cranelift_frontend::FunctionBuilder;

use crate::ast::{Argument, Expr};

use super::expressions::coerce_to_ir_type;
use super::expressions::{initializer_type, lower_expression};
use super::layout::LayoutRegistry;
use super::literals::StringDataValues;
use super::model::NativeCleanupSchedule;
use super::native::{FunctionRef, NativeEmitError};
use super::structs::{copy_bytes, expression_native_type};
use super::types::NativeType;

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_place(
    function: &mut FunctionBuilder<'_>,
    receiver: &Expr,
    arguments: &[Argument],
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let (argument, capacity, size, alignment) =
        place_inputs(receiver, arguments, local_types, functions, layouts)?;
    let arena = lower_expression(
        function,
        receiver,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    let value = lower_expression(
        function,
        argument,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    lower_placed_value(function, arena, value, capacity, size, alignment, layouts)
}

fn place_inputs<'a>(
    receiver: &Expr,
    arguments: &'a [Argument],
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<(&'a Expr, u32, u32, u32), NativeEmitError> {
    let argument = arguments
        .first()
        .map(|argument| &argument.expression)
        .ok_or_else(|| NativeEmitError("place requires one value".to_owned()))?;
    let (capacity, size, alignment) =
        placement_layout(receiver, argument, local_types, functions, layouts)?;
    Ok((argument, capacity, size, alignment))
}

fn placement_layout(
    receiver: &Expr,
    argument: &Expr,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<(u32, u32, u32), NativeEmitError> {
    let NativeType::Arena(capacity) = expression_native_type(receiver, local_types, layouts)
        .ok_or_else(|| NativeEmitError("place receiver is not an Arena".to_owned()))?
    else {
        return Err(NativeEmitError("place receiver is not an Arena".to_owned()));
    };
    let value_type = initializer_type(argument, local_types, functions, layouts)?;
    let size = layouts
        .type_size(value_type)
        .ok_or_else(|| NativeEmitError("placed value has no native layout".to_owned()))?;
    Ok((capacity, size, layouts.alignment(value_type)?.max(1)))
}

fn lower_placed_value(
    function: &mut FunctionBuilder<'_>,
    arena: cranelift_codegen::ir::Value,
    value: cranelift_codegen::ir::Value,
    capacity: u32,
    size: u32,
    alignment: u32,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let offset_address = function.ins().iadd_imm_s(arena, i64::from(capacity));
    let offset = function.ins().load(layouts.pointer_type, MemFlagsData::new(), offset_address, 0);
    let aligned = aligned_offset(function, offset, alignment, layouts);
    let new_offset = function.ins().iadd_imm_s(aligned, i64::from(size));
    let fits = capacity_check(function, new_offset, capacity, layouts);
    let ok_block = function.create_block();
    let trap_block = function.create_block();
    function.ins().brif(fits, ok_block, &[], trap_block, &[]);
    function.switch_to_block(trap_block);
    function.ins().trap(TrapCode::HEAP_OUT_OF_BOUNDS);
    function.seal_block(trap_block);
    function.switch_to_block(ok_block);
    function.ins().store(MemFlagsData::new(), new_offset, offset_address, 0);
    let destination = function.ins().iadd(arena, aligned);
    copy_bytes(function, value, destination, size);
    function.seal_block(ok_block);
    Ok(coerce_to_ir_type(function, destination, layouts.pointer_type))
}

fn aligned_offset(
    function: &mut FunctionBuilder<'_>,
    offset: cranelift_codegen::ir::Value,
    alignment: u32,
    layouts: &LayoutRegistry,
) -> cranelift_codegen::ir::Value {
    let alignment_minus_one =
        function.ins().iconst(layouts.pointer_type, i64::from(alignment.saturating_sub(1)));
    let rounded = function.ins().iadd(offset, alignment_minus_one);
    let alignment_mask = function.ins().bnot(alignment_minus_one);
    function.ins().band(rounded, alignment_mask)
}

fn capacity_check(
    function: &mut FunctionBuilder<'_>,
    new_offset: cranelift_codegen::ir::Value,
    capacity: u32,
    layouts: &LayoutRegistry,
) -> cranelift_codegen::ir::Value {
    let capacity_value = function.ins().iconst(layouts.pointer_type, i64::from(capacity));
    function.ins().icmp(IntCC::UnsignedLessThanOrEqual, new_offset, capacity_value)
}
