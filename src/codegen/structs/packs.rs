use std::collections::HashMap;

use cranelift_codegen::ir::InstBuilder;
use cranelift_frontend::FunctionBuilder;

use crate::ast::{Expr, StructFieldInit};

use super::super::expressions::lower_expression;
use super::super::layout::{LayoutRegistry, PackFieldLayout};
use super::super::literals::StringDataValues;
use super::super::model::NativeCleanupSchedule;
use super::super::native::{FunctionRef, NativeEmitError};
use super::super::types::NativeType;

#[allow(clippy::too_many_arguments)]
pub(crate) fn lower_pack_literal(
    function: &mut FunctionBuilder<'_>,
    name: &str,
    fields: &[StructFieldInit],
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let id =
        layouts.pack_id(name).ok_or_else(|| NativeEmitError(format!("missing pack `{name}`")))?;
    let pack =
        layouts.pack(id).ok_or_else(|| NativeEmitError("missing packed layout".to_owned()))?;
    let storage = fields
        .iter()
        .find(|field| field.name == "storage")
        .ok_or_else(|| NativeEmitError("pack literal is missing storage".to_owned()))?;
    let value = lower_expression(
        function,
        &storage.value,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    Ok(super::super::expressions::coerce_to_ir_type(
        function,
        value,
        layouts.ir_type(pack.storage)?,
    ))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn lower_pack_field_assignment(
    function: &mut FunctionBuilder<'_>,
    object: &Expr,
    field: &str,
    value: &Expr,
    pack_id: usize,
    locals: &mut HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    let name = binding_name(object)?;
    let pack =
        layouts.pack(pack_id).ok_or_else(|| NativeEmitError("missing packed layout".to_owned()))?;
    let field_layout = packed_field(pack, field)?;
    if !matches!(field_layout.role, crate::ast::StructFieldRole::Erg) {
        return Err(NativeEmitError(format!("packed field `{field}` is read-only")));
    }
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
    let new_value = lower_expression(
        function,
        value,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    let updated = lower_pack_field_write(
        function,
        storage,
        new_value,
        field_layout,
        pack.storage,
        pack.endianness,
        layouts,
    )?;
    let binding = locals
        .keys()
        .find(|candidate| candidate.as_str() == name)
        .copied()
        .ok_or_else(|| NativeEmitError(format!("native binding `{name}` is unavailable")))?;
    locals.insert(binding, updated);
    Ok(())
}

pub(crate) fn lower_pack_field(
    function: &mut FunctionBuilder<'_>,
    storage: cranelift_codegen::ir::Value,
    pack_id: usize,
    field: &str,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let pack =
        layouts.pack(pack_id).ok_or_else(|| NativeEmitError("missing packed layout".to_owned()))?;
    let field_layout = packed_field(pack, field)?;
    let storage_type = layouts.ir_type(pack.storage)?;
    let bit_offset = mapped_bit_offset(pack, field_layout, layouts)?;
    let shifted = if bit_offset == 0 {
        storage
    } else {
        let offset = function.ins().iconst(storage_type, i64::from(bit_offset));
        function.ins().ushr(storage, offset)
    };
    let mask_value = function.ins().iconst(storage_type, bit_mask(field_layout.width));
    let masked = function.ins().band(shifted, mask_value);
    Ok(super::super::expressions::coerce_to_ir_type(
        function,
        masked,
        layouts.ir_type(field_layout.ty)?,
    ))
}

fn lower_pack_field_write(
    function: &mut FunctionBuilder<'_>,
    storage: cranelift_codegen::ir::Value,
    new_value: cranelift_codegen::ir::Value,
    field: &PackFieldLayout,
    storage_native_type: NativeType,
    endianness: crate::ast::LayoutEndianness,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let storage_type = layouts.ir_type(storage_native_type)?;
    let mask_value = function.ins().iconst(storage_type, bit_mask(field.width));
    let bit_offset = mapped_bit_offset_for_storage(
        field,
        endianness,
        layouts
            .type_size(storage_native_type)
            .ok_or_else(|| NativeEmitError("missing packed storage size".to_owned()))?,
    )?;
    let shifted_mask = shift_value(function, mask_value, bit_offset, storage_type);
    let inverse_mask = function.ins().bnot(shifted_mask);
    let cleared = function.ins().band(storage, inverse_mask);
    let value = super::super::expressions::coerce_to_ir_type(function, new_value, storage_type);
    let value = function.ins().band(value, mask_value);
    let value = shift_value(function, value, bit_offset, storage_type);
    Ok(function.ins().bor(cleared, value))
}

fn shift_value(
    function: &mut FunctionBuilder<'_>,
    value: cranelift_codegen::ir::Value,
    offset: u16,
    storage_type: cranelift_codegen::ir::Type,
) -> cranelift_codegen::ir::Value {
    if offset == 0 {
        value
    } else {
        let amount = function.ins().iconst(storage_type, i64::from(offset));
        function.ins().ishl(value, amount)
    }
}

fn packed_field<'a>(
    pack: &'a super::super::layout::PackLayout,
    field: &str,
) -> Result<&'a PackFieldLayout, NativeEmitError> {
    pack.fields
        .iter()
        .find(|candidate| candidate.name == field)
        .ok_or_else(|| NativeEmitError(format!("unknown packed field `{field}`")))
}

fn bit_mask(width: u8) -> i64 {
    if width >= 64 { -1 } else { (1_i64 << width) - 1 }
}

fn mapped_bit_offset(
    pack: &super::super::layout::PackLayout,
    field: &PackFieldLayout,
    layouts: &LayoutRegistry,
) -> Result<u16, NativeEmitError> {
    let size = layouts
        .type_size(pack.storage)
        .ok_or_else(|| NativeEmitError("missing packed storage size".to_owned()))?;
    mapped_bit_offset_for_storage(field, pack.endianness, size)
}

fn mapped_bit_offset_for_storage(
    field: &PackFieldLayout,
    endianness: crate::ast::LayoutEndianness,
    storage_size: u32,
) -> Result<u16, NativeEmitError> {
    let capacity = storage_size
        .checked_mul(8)
        .ok_or_else(|| NativeEmitError("packed storage size overflow".to_owned()))?;
    match endianness {
        crate::ast::LayoutEndianness::Little => Ok(field.offset),
        crate::ast::LayoutEndianness::Big => capacity
            .checked_sub(u32::from(field.offset) + u32::from(field.width))
            .and_then(|offset| u16::try_from(offset).ok())
            .ok_or_else(|| NativeEmitError("packed field offset exceeds storage".to_owned())),
    }
}

fn binding_name(expression: &Expr) -> Result<&String, NativeEmitError> {
    match expression {
        Expr::Identifier { name, .. } => Ok(name),
        _ => Err(NativeEmitError("packed assignment requires a direct binding".to_owned())),
    }
}
