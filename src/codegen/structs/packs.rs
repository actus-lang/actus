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
    lower_pack_storage_value(
        function,
        storage,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        pack.storage,
        layouts,
    )
}

#[allow(clippy::too_many_arguments)]
fn lower_pack_storage_value(
    function: &mut FunctionBuilder<'_>,
    storage: &StructFieldInit,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    storage_type: NativeType,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
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
        layouts.ir_type(storage_type)?,
    ))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn lower_pack_field_assignment<'source>(
    function: &mut FunctionBuilder<'_>,
    object: &Expr,
    field: &str,
    value: &Expr,
    pack_id: usize,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    lower_pack_field_assignment_inner(
        PackAssignmentContext {
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
        },
        pack_id,
    )
}

struct PackAssignmentContext<'input, 'source, 'function> {
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

fn lower_pack_field_assignment_inner(
    context: PackAssignmentContext<'_, '_, '_>,
    pack_id: usize,
) -> Result<(), NativeEmitError> {
    if let Expr::Index { target, index, .. } = context.object {
        return lower_indexed_pack_field_assignment(context, pack_id, target, index);
    }
    let name = binding_name(context.object)?;
    let pack = context
        .layouts
        .pack(pack_id)
        .ok_or_else(|| NativeEmitError("missing packed layout".to_owned()))?;
    let field_layout = packed_field(pack, context.field)?;
    ensure_pack_field_is_mutable(field_layout, context.field)?;
    let (storage, new_value) = lower_pack_assignment_operands(
        context.function,
        context.object,
        context.value,
        context.locals,
        context.local_types,
        context.functions,
        context.cleanup_schedule,
        context.string_data,
        context.layouts,
    )?;
    let updated = lower_pack_field_write(
        context.function,
        storage,
        new_value,
        field_layout,
        pack.storage,
        pack.endianness,
        context.layouts,
    )?;
    update_pack_binding(context.locals, name, updated)?;
    Ok(())
}

fn lower_indexed_pack_field_assignment(
    context: PackAssignmentContext<'_, '_, '_>,
    pack_id: usize,
    target: &Expr,
    index: &Expr,
) -> Result<(), NativeEmitError> {
    let (address, element) = super::super::arrays::lower_array_address(
        context.function,
        target,
        index,
        context.locals,
        context.local_types,
        context.functions,
        context.cleanup_schedule,
        context.string_data,
        context.layouts,
    )?;
    if element != NativeType::Pack(pack_id) {
        return Err(NativeEmitError(
            "indexed pack layout does not match field assignment".to_owned(),
        ));
    }
    let pack = context
        .layouts
        .pack(pack_id)
        .ok_or_else(|| NativeEmitError("missing packed layout".to_owned()))?;
    let field_layout = packed_field(pack, context.field)?;
    ensure_pack_field_is_mutable(field_layout, context.field)?;
    let storage = context.function.ins().load(
        context.layouts.ir_type(pack.storage)?,
        cranelift_codegen::ir::MemFlagsData::new(),
        address,
        0,
    );
    let new_value = lower_expression(
        context.function,
        context.value,
        context.locals,
        context.local_types,
        context.functions,
        context.cleanup_schedule,
        context.string_data,
        context.layouts,
    )?;
    let updated = lower_pack_field_write(
        context.function,
        storage,
        new_value,
        field_layout,
        pack.storage,
        pack.endianness,
        context.layouts,
    )?;
    context.function.ins().store(cranelift_codegen::ir::MemFlagsData::new(), updated, address, 0);
    Ok(())
}

fn update_pack_binding(
    locals: &mut HashMap<&String, cranelift_codegen::ir::Value>,
    name: &str,
    updated: cranelift_codegen::ir::Value,
) -> Result<(), NativeEmitError> {
    let binding = locals
        .keys()
        .find(|candidate| candidate.as_str() == name)
        .copied()
        .ok_or_else(|| NativeEmitError(format!("native binding `{name}` is unavailable")))?;
    locals.insert(binding, updated);
    Ok(())
}

fn ensure_pack_field_is_mutable(
    field_layout: &PackFieldLayout,
    field: &str,
) -> Result<(), NativeEmitError> {
    if !matches!(field_layout.role, crate::ast::StructFieldRole::Erg) {
        return Err(NativeEmitError(format!("packed field `{field}` is read-only")));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn lower_pack_assignment_operands(
    function: &mut FunctionBuilder<'_>,
    object: &Expr,
    value: &Expr,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<(cranelift_codegen::ir::Value, cranelift_codegen::ir::Value), NativeEmitError> {
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
    Ok((storage, new_value))
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
    let storage_bits = packed_storage_bits(pack.storage, layouts)?;
    let mask_value = field_mask(function, field_layout.width, storage_bits, storage_type);
    let masked = function.ins().band(shifted, mask_value);
    let extracted = sign_extend_field(
        function,
        masked,
        field_layout.ty,
        field_layout.width,
        storage_bits,
        storage_type,
    );
    Ok(super::super::expressions::coerce_to_ir_type(
        function,
        extracted,
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
    let storage_bits = packed_storage_bits(storage_native_type, layouts)?;
    let mask_value = field_mask(function, field.width, storage_bits, storage_type);
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

fn field_mask(
    function: &mut FunctionBuilder<'_>,
    width: u8,
    storage_bits: u16,
    storage_type: cranelift_codegen::ir::Type,
) -> cranelift_codegen::ir::Value {
    if u16::from(width) >= storage_bits {
        return function.ins().iconst(storage_type, -1);
    }
    let one = function.ins().iconst(storage_type, 1);
    let shift = function.ins().iconst(storage_type, i64::from(width));
    let shifted = function.ins().ishl(one, shift);
    let one_again = function.ins().iconst(storage_type, 1);
    function.ins().isub(shifted, one_again)
}

fn sign_extend_field(
    function: &mut FunctionBuilder<'_>,
    value: cranelift_codegen::ir::Value,
    field_type: NativeType,
    width: u8,
    storage_bits: u16,
    storage_type: cranelift_codegen::ir::Type,
) -> cranelift_codegen::ir::Value {
    let NativeType::Integer { signed: true, .. } = field_type else { return value };
    if u16::from(width) >= storage_bits {
        return value;
    }
    let shift_amount = storage_bits - u16::from(width);
    let shift = function.ins().iconst(storage_type, i64::from(shift_amount));
    let shifted = function.ins().ishl(value, shift);
    function.ins().sshr(shifted, shift)
}

fn packed_storage_bits(
    storage: NativeType,
    layouts: &LayoutRegistry,
) -> Result<u16, NativeEmitError> {
    let bytes = layouts
        .type_size(storage)
        .ok_or_else(|| NativeEmitError("missing packed storage size".to_owned()))?;
    u16::try_from(
        bytes
            .checked_mul(8)
            .ok_or_else(|| NativeEmitError("packed storage bit width overflow".to_owned()))?,
    )
    .map_err(|_| NativeEmitError("packed storage bit width exceeds u16".to_owned()))
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
