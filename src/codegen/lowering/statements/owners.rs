use std::collections::HashMap;

use cranelift_codegen::ir::InstBuilder;
use cranelift_frontend::FunctionBuilder;

use crate::ast::Expr;

use super::super::super::expressions::{initializer_type, lower_expression};
use super::super::super::layout::LayoutRegistry;
use super::super::super::literals::StringDataValues;
use super::super::super::native::{FunctionRef, NativeEmitError};
use super::super::super::types::NativeType;
use super::super::Flow;

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_owner_declaration<'source>(
    function: &mut FunctionBuilder<'_>,
    name: &'source String,
    declared_type: Option<&str>,
    initializer: &Expr,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &mut HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &super::super::NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    let declared_native = declared_native_type(declared_type, layouts);
    let native_type = match declared_native {
        Some(native_type) => native_type,
        None => initializer_type(initializer, types, functions, layouts)?,
    };
    if let NativeType::Arena(capacity) = native_type {
        let address = lower_arena_declaration(function, capacity, layouts)?;
        locals.insert(name, address);
        types.insert(name, native_type);
        return Ok(Flow::Fallthrough);
    }
    let value = lower_owner_value(
        function,
        declared_native,
        initializer,
        locals,
        types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    store_owner_binding(name, value, native_type, locals, types);
    Ok(Flow::Fallthrough)
}

fn store_owner_binding<'source>(
    name: &'source String,
    value: cranelift_codegen::ir::Value,
    native_type: NativeType,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &mut HashMap<&'source String, NativeType>,
) {
    locals.insert(name, value);
    types.insert(name, native_type);
}

fn declared_native_type(
    declared_type: Option<&str>,
    layouts: &LayoutRegistry,
) -> Option<NativeType> {
    declared_type.and_then(|name| {
        NativeType::from_name(name)
            .or_else(|| layouts.type_for_name(name))
            .or_else(|| layouts.array_id(name).map(NativeType::Array))
    })
}

#[allow(clippy::too_many_arguments)]
fn lower_owner_value(
    function: &mut FunctionBuilder<'_>,
    declared_native: Option<NativeType>,
    initializer: &Expr,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &super::super::NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let value = lower_initializer_value(
        function,
        declared_native,
        initializer,
        locals,
        types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    coerce_owner_value(function, value, declared_native, layouts)
}

fn coerce_owner_value(
    function: &mut FunctionBuilder<'_>,
    value: cranelift_codegen::ir::Value,
    declared_native: Option<NativeType>,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    match declared_native {
        Some(declared) => Ok(super::super::super::expressions::coerce_to_ir_type(
            function,
            value,
            layouts.ir_type(declared)?,
        )),
        None => Ok(value),
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_initializer_value(
    function: &mut FunctionBuilder<'_>,
    declared_native: Option<NativeType>,
    initializer: &Expr,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &super::super::NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    if let Some(value) = lower_special_initializer(function, declared_native, initializer, layouts)?
    {
        return Ok(value);
    }
    lower_expression(
        function,
        initializer,
        locals,
        types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )
}

fn lower_special_initializer(
    function: &mut FunctionBuilder<'_>,
    declared_native: Option<NativeType>,
    initializer: &Expr,
    layouts: &LayoutRegistry,
) -> Result<Option<cranelift_codegen::ir::Value>, NativeEmitError> {
    let value = match (declared_native, initializer) {
        (Some(NativeType::Array(id)), Expr::Call { callee, arguments, .. })
            if arguments.is_empty() =>
        {
            let expected = layouts
                .array_id(callee)
                .ok_or_else(|| NativeEmitError(format!("missing array layout for `{callee}`")))?;
            if expected != id {
                return Err(NativeEmitError(
                    "array initializer type does not match binding".to_owned(),
                ));
            }
            Some(super::super::super::arrays::lower_array_constructor(function, id, layouts)?)
        }
        (Some(NativeType::Integer { width: 65..=128, .. }), Expr::Integer { value, .. }) => {
            Some(super::super::super::expressions::lower_wide_integer(function, value)?)
        }
        (Some(NativeType::Float { width }), Expr::FloatLiteral { value, .. }) => {
            Some(super::super::super::expressions::lower_float_as(
                function,
                value,
                layouts.ir_type(NativeType::Float { width })?,
            )?)
        }
        _ => None,
    };
    Ok(value)
}

fn lower_arena_declaration(
    function: &mut FunctionBuilder<'_>,
    capacity: u32,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let size = capacity
        .checked_add(layouts.pointer_type.bytes())
        .ok_or_else(|| NativeEmitError("arena storage size overflow".to_owned()))?;
    let slot = function.func.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
        cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
        size,
        layouts.pointer_type.bytes().trailing_zeros() as u8,
    ));
    let address = function.ins().stack_addr(layouts.pointer_type, slot, 0);
    let offset_address = function.ins().iadd_imm_s(address, i64::from(capacity));
    let zero = function.ins().iconst(layouts.pointer_type, 0);
    function.ins().store(cranelift_codegen::ir::MemFlagsData::new(), zero, offset_address, 0);
    Ok(address)
}
