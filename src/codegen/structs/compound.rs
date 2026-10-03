use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, Value};
use cranelift_frontend::FunctionBuilder;

use crate::ast::{BinaryOp, Expr};

use super::super::expressions::{lower_compound_integer_operation, lower_expression};
use super::super::layout::LayoutRegistry;
use super::super::literals::StringDataValues;
use super::super::model::NativeCleanupSchedule;
use super::super::native::{FunctionRef, NativeEmitError};
use super::super::types::NativeType;
use super::pack_state::{seal_checked_pack_block, update_pack_binding};
use super::packs::{
    binding_name, ensure_pack_field_is_mutable, lower_pack_field, lower_pack_field_write,
    packed_field,
};

#[allow(clippy::too_many_arguments)]
pub(crate) fn lower_pack_field_compound_assignment<'source>(
    function: &mut FunctionBuilder<'_>,
    object: &Expr,
    field: &str,
    operator: BinaryOp,
    value: &Expr,
    pack_id: usize,
    locals: &mut HashMap<&'source String, Value>,
    local_types: &HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    if let Expr::Index { target, index, .. } = object {
        return lower_indexed_pack_field_compound_assignment(
            function,
            target,
            index,
            field,
            operator,
            value,
            pack_id,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        );
    }
    lower_direct_pack_field_compound_assignment(
        function,
        object,
        field,
        operator,
        value,
        pack_id,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )
}

#[allow(clippy::too_many_arguments)]
fn lower_direct_pack_field_compound_assignment<'source>(
    function: &mut FunctionBuilder<'_>,
    object: &Expr,
    field: &str,
    operator: BinaryOp,
    value: &Expr,
    pack_id: usize,
    locals: &mut HashMap<&'source String, Value>,
    local_types: &HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    let name = binding_name(object)?;
    let pack =
        layouts.pack(pack_id).ok_or_else(|| NativeEmitError("missing packed layout".to_owned()))?;
    let field_layout = packed_field(pack, field)?;
    ensure_pack_field_is_mutable(field_layout, field)?;
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
    let current = lower_pack_field(function, storage, pack_id, field, layouts)?;
    let right = lower_pack_compound_rhs(
        function,
        value,
        field_layout.ty,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    let updated =
        lower_compound_integer_operation(function, current, operator, right, field_layout.ty)?;
    let storage = lower_pack_field_write(
        function,
        storage,
        updated,
        field_layout,
        pack.storage,
        pack.endianness,
        layouts,
    )?;
    if !matches!(pack.storage, NativeType::Array(_)) {
        seal_checked_pack_block(function, field_layout.ty, "pack compound write block");
    }
    update_pack_binding(locals, name, storage)
}

#[allow(clippy::too_many_arguments)]
fn lower_indexed_pack_field_compound_assignment<'source>(
    function: &mut FunctionBuilder<'_>,
    target: &Expr,
    index: &Expr,
    field: &str,
    operator: BinaryOp,
    value: &Expr,
    pack_id: usize,
    locals: &mut HashMap<&'source String, Value>,
    local_types: &HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    let (address, element) = super::super::arrays::lower_array_address(
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
    if element != NativeType::Pack(pack_id) {
        return Err(NativeEmitError(
            "indexed pack layout does not match field assignment".to_owned(),
        ));
    }
    lower_indexed_pack_compound_at_address(
        function,
        address,
        field,
        operator,
        value,
        pack_id,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )
}

#[allow(clippy::too_many_arguments)]
fn lower_indexed_pack_compound_at_address<'source>(
    function: &mut FunctionBuilder<'_>,
    address: Value,
    field: &str,
    operator: BinaryOp,
    value: &Expr,
    pack_id: usize,
    locals: &mut HashMap<&'source String, Value>,
    local_types: &HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    let pack =
        layouts.pack(pack_id).ok_or_else(|| NativeEmitError("missing packed layout".to_owned()))?;
    let field_layout = packed_field(pack, field)?;
    ensure_pack_field_is_mutable(field_layout, field)?;
    let storage = load_indexed_pack_storage(function, address, pack.storage, layouts)?;
    let current = lower_pack_field(function, storage, pack_id, field, layouts)?;
    let right = lower_pack_compound_rhs(
        function,
        value,
        field_layout.ty,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    let updated =
        lower_compound_integer_operation(function, current, operator, right, field_layout.ty)?;
    let updated_storage = lower_pack_field_write(
        function,
        storage,
        updated,
        field_layout,
        pack.storage,
        pack.endianness,
        layouts,
    )?;
    if !matches!(pack.storage, NativeType::Array(_)) {
        function.ins().store(
            cranelift_codegen::ir::MemFlagsData::new(),
            updated_storage,
            address,
            0,
        );
        seal_checked_pack_block(function, field_layout.ty, "indexed pack compound write block");
    }
    Ok(())
}

fn load_indexed_pack_storage(
    function: &mut FunctionBuilder<'_>,
    address: Value,
    storage_type: NativeType,
    layouts: &LayoutRegistry,
) -> Result<Value, NativeEmitError> {
    if matches!(storage_type, NativeType::Array(_)) {
        return Ok(address);
    }
    Ok(function.ins().load(
        layouts.ir_type(storage_type)?,
        cranelift_codegen::ir::MemFlagsData::new(),
        address,
        0,
    ))
}

#[allow(clippy::too_many_arguments)]
fn lower_pack_compound_rhs(
    function: &mut FunctionBuilder<'_>,
    value: &Expr,
    field_type: NativeType,
    locals: &HashMap<&String, Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Value, NativeEmitError> {
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
    Ok(super::super::expressions::coerce_to_ir_type(function, value, layouts.ir_type(field_type)?))
}
