use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData};
use cranelift_frontend::FunctionBuilder;

use crate::ast::Expr;

use super::super::expressions::lower_expression;
use super::super::layout::LayoutRegistry;
use super::super::literals::StringDataValues;
use super::super::model::NativeCleanupSchedule;
use super::super::native::{FunctionRef, NativeEmitError};
use super::super::types::NativeType;
use super::memory::copy_bytes;
use super::types::expression_native_type;

#[allow(clippy::too_many_arguments)]
pub(crate) fn lower_field_access(
    function: &mut FunctionBuilder<'_>,
    object: &Expr,
    field: &str,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    if let Some(NativeType::Pack(id)) = expression_native_type(object, local_types, layouts) {
        return lower_pack_field_access(
            function,
            object,
            field,
            id,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        );
    }
    lower_struct_field_access(
        function,
        object,
        field,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )
}

#[allow(clippy::too_many_arguments)]
fn lower_pack_field_access(
    function: &mut FunctionBuilder<'_>,
    object: &Expr,
    field: &str,
    id: usize,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let storage = lower_expression(
        function,
        object,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    super::packs::lower_pack_field(function, storage, id, field, layouts)
}

#[allow(clippy::too_many_arguments)]
fn lower_struct_field_access(
    function: &mut FunctionBuilder<'_>,
    object: &Expr,
    field: &str,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let (address, field_layout) = lower_field_address(
        function,
        object,
        field,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    load_struct_field(function, address, &field_layout, layouts)
}

fn load_struct_field(
    function: &mut FunctionBuilder<'_>,
    address: cranelift_codegen::ir::Value,
    field_layout: &super::super::layout::FieldLayout,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    if field_layout.indirect {
        return Ok(function.ins().load(
            layouts.pointer_type,
            MemFlagsData::new(),
            address,
            field_layout.offset as i32,
        ));
    }
    if matches!(field_layout.ty, NativeType::Struct(_)) {
        return Ok(function.ins().iadd_imm_s(address, i64::from(field_layout.offset)));
    }
    Ok(function.ins().load(
        layouts.ir_type(field_layout.ty)?,
        MemFlagsData::new(),
        address,
        field_layout.offset as i32,
    ))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn lower_field_assignment<'source>(
    function: &mut FunctionBuilder<'_>,
    object: &Expr,
    field: &str,
    value: &Expr,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    let context = FieldAssignmentContext {
        function,
        object,
        field,
        value,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    };
    match expression_native_type(object, local_types, layouts) {
        Some(NativeType::Pack(id)) => lower_pack_field_assignment(context, id),
        _ => lower_struct_field_assignment(context),
    }
}

struct FieldAssignmentContext<'input, 'source, 'function> {
    function: &'input mut FunctionBuilder<'function>,
    object: &'input Expr,
    field: &'input str,
    value: &'input Expr,
    locals: &'input mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    local_types: &'input HashMap<&'source String, NativeType>,
    functions: &'input HashMap<String, FunctionRef>,
    cleanup_schedule: &'input NativeCleanupSchedule,
    string_data: &'input StringDataValues,
    layouts: &'input LayoutRegistry,
}

fn lower_pack_field_assignment(
    context: FieldAssignmentContext<'_, '_, '_>,
    id: usize,
) -> Result<(), NativeEmitError> {
    super::packs::lower_pack_field_assignment(
        context.function,
        context.object,
        context.field,
        context.value,
        id,
        context.locals,
        context.local_types,
        context.functions,
        context.cleanup_schedule,
        context.string_data,
        context.layouts,
    )
}

fn lower_struct_field_assignment(
    context: FieldAssignmentContext<'_, '_, '_>,
) -> Result<(), NativeEmitError> {
    let (address, field_layout) = lower_field_address(
        context.function,
        context.object,
        context.field,
        context.locals,
        context.local_types,
        context.functions,
        context.cleanup_schedule,
        context.string_data,
        context.layouts,
    )?;
    let value = lower_expression(
        context.function,
        context.value,
        context.locals,
        context.local_types,
        context.functions,
        context.cleanup_schedule,
        context.string_data,
        context.layouts,
    )?;
    store_struct_field(context.function, address, value, &field_layout, context.layouts)
}

fn store_struct_field(
    function: &mut FunctionBuilder<'_>,
    address: cranelift_codegen::ir::Value,
    value: cranelift_codegen::ir::Value,
    field_layout: &super::super::layout::FieldLayout,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    if field_layout.indirect
        || matches!(field_layout.ty, NativeType::Enum(id) if layouts.is_niche_option(id))
    {
        function.ins().store(MemFlagsData::new(), value, address, field_layout.offset as i32);
    } else if matches!(field_layout.ty, NativeType::Struct(_) | NativeType::Enum(_)) {
        let size = layouts
            .type_size(field_layout.ty)
            .ok_or_else(|| NativeEmitError("missing nested field layout".to_owned()))?;
        let destination = function.ins().iadd_imm_s(address, i64::from(field_layout.offset));
        copy_bytes(function, value, destination, size);
    } else {
        function.ins().store(MemFlagsData::new(), value, address, field_layout.offset as i32);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn lower_field_address(
    function: &mut FunctionBuilder<'_>,
    object: &Expr,
    field: &str,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<(cranelift_codegen::ir::Value, super::super::layout::FieldLayout), NativeEmitError> {
    let object_type = expression_native_type(object, local_types, layouts)
        .ok_or_else(|| NativeEmitError("field access requires a struct value".to_owned()))?;
    let NativeType::Struct(id) = object_type else {
        return Err(NativeEmitError("field access requires a struct value".to_owned()));
    };
    let layout =
        layouts.get(id).ok_or_else(|| NativeEmitError(format!("missing layout `{id}`")))?;
    let field_layout = layout
        .fields
        .iter()
        .find(|candidate| candidate.name == field)
        .cloned()
        .ok_or_else(|| NativeEmitError(format!("unknown native field `{field}`")))?;
    let address = lower_expression(
        function,
        object,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    Ok((address, field_layout))
}
