use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData, Value, condcodes::IntCC};
use cranelift_frontend::FunctionBuilder;

use crate::ast::{BinaryOp, Expr};

use super::expressions::lower_expression;
use super::layout::LayoutRegistry;
use super::literals::StringDataValues;
use super::model::NativeCleanupSchedule;
use super::native::{FunctionRef, NativeEmitError};
use super::types::NativeType;

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_buffer_index(
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
    let address = lower_buffer_address(
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
    let byte =
        function.ins().load(cranelift_codegen::ir::types::I8, MemFlagsData::new(), address, 0);
    Ok(byte)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_buffer_compound_assignment(
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
    let address = lower_buffer_address(
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
    let current =
        function.ins().load(cranelift_codegen::ir::types::I8, MemFlagsData::new(), address, 0);
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
    let right =
        super::expressions::coerce_to_ir_type(function, right, cranelift_codegen::ir::types::I8);
    let updated = super::expressions::lower_compound_integer_operation(
        function,
        current,
        operator,
        right,
        NativeType::Integer { signed: false, width: 8 },
    )?;
    function.ins().store(MemFlagsData::new(), updated, address, 0);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn lower_buffer_address(
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
    let handle = lower_expression(
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
    let length = function.ins().load(
        layouts.pointer_type,
        MemFlagsData::new(),
        handle,
        layouts.pointer_size as i32,
    );
    let in_bounds = function.ins().icmp(IntCC::UnsignedLessThan, index, length);
    let ok_block = function.create_block();
    let trap_block = function.create_block();
    function.ins().brif(in_bounds, ok_block, &[], trap_block, &[]);
    function.switch_to_block(trap_block);
    function.ins().trap(cranelift_codegen::ir::TrapCode::HEAP_OUT_OF_BOUNDS);
    function.seal_block(trap_block);
    function.switch_to_block(ok_block);
    let data = function.ins().load(layouts.pointer_type, MemFlagsData::new(), handle, 0);
    let address = function.ins().iadd(data, index);
    function.seal_block(ok_block);
    Ok(address)
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
