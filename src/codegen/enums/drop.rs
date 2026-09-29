use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData};
use cranelift_frontend::FunctionBuilder;

use super::super::enum_layout::{EnumFieldLayout, EnumLayout, EnumVariantLayout};
use super::super::layout::LayoutRegistry;
use super::super::native::{FunctionRef, NativeEmitError};
use super::super::structs::emit_struct_drop;
use super::super::types::NativeType;

#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_enum_payload_drop(
    function: &mut FunctionBuilder<'_>,
    subject: &str,
    enum_name: &str,
    variant: &str,
    field: &str,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    let address = enum_subject_address(locals, subject)?;
    let (enum_layout, variant_layout) =
        payload_layouts(types, subject, enum_name, variant, layouts)?;
    if enum_layout.niche_pointer {
        return Ok(());
    }
    let field_layout = payload_field(variant_layout, field)?;
    let field_address = function
        .ins()
        .iadd_imm_s(address, i64::from(enum_layout.payload_offset + field_layout.offset));
    drop_enum_payload_field(function, field_layout.ty, field_address, functions, layouts)
}

fn payload_layouts<'a>(
    types: &HashMap<&String, NativeType>,
    subject: &str,
    enum_name: &str,
    variant: &str,
    layouts: &'a LayoutRegistry,
) -> Result<(&'a EnumLayout, &'a EnumVariantLayout), NativeEmitError> {
    let enum_id = types
        .iter()
        .find(|(binding, ty)| binding.as_str() == subject && matches!(ty, NativeType::Enum(_)))
        .and_then(|(_, ty)| match ty {
            NativeType::Enum(id) => Some(*id),
            _ => None,
        })
        .or_else(|| layouts.enum_constructor(enum_name, variant).map(|(id, _)| id))
        .ok_or_else(|| NativeEmitError(format!("unknown enum payload `{enum_name}.{variant}`")))?;
    let variant_layout = layouts
        .enum_variant(enum_id, variant)
        .ok_or_else(|| NativeEmitError(format!("unknown enum payload `{enum_name}.{variant}`")))?;
    let enum_layout = layouts
        .enum_layout(enum_id)
        .ok_or_else(|| NativeEmitError(format!("missing enum layout `{enum_id}`")))?;
    Ok((enum_layout, variant_layout))
}

fn enum_subject_address(
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    subject: &str,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    locals
        .iter()
        .find(|(binding, _)| binding.as_str() == subject)
        .map(|(_, value)| *value)
        .ok_or_else(|| NativeEmitError(format!("native binding `{subject}` is unavailable")))
}

fn drop_enum_payload_field(
    function: &mut FunctionBuilder<'_>,
    field_type: NativeType,
    field_address: cranelift_codegen::ir::Value,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    match field_type {
        NativeType::Buffer => drop_buffer_field(function, field_address, functions, layouts)?,
        NativeType::Struct(nested_id) => {
            emit_struct_drop(function, field_address, nested_id, functions, layouts)?
        }
        NativeType::Int
        | NativeType::Integer { .. }
        | NativeType::Float { .. }
        | NativeType::Void
        | NativeType::String
        | NativeType::Enum(_)
        | NativeType::Pack(_)
        | NativeType::Array(_)
        | NativeType::Arena(_)
        | NativeType::FatPointer => {}
    }
    Ok(())
}

fn drop_buffer_field(
    function: &mut FunctionBuilder<'_>,
    field_address: cranelift_codegen::ir::Value,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    let handle = function.ins().load(layouts.pointer_type, MemFlagsData::new(), field_address, 0);
    let target = functions.get("actus_buffer_drop").ok_or_else(|| {
        NativeEmitError("native runtime function `actus_buffer_drop` is unavailable".to_owned())
    })?;
    function.ins().call(target.reference, &[handle]);
    Ok(())
}

fn payload_field<'a>(
    variant: &'a EnumVariantLayout,
    field: &str,
) -> Result<&'a EnumFieldLayout, NativeEmitError> {
    if let Ok(index) = field.parse::<usize>() {
        return variant
            .fields
            .get(index)
            .ok_or_else(|| NativeEmitError(format!("unknown tuple payload field `{field}`")));
    }
    variant
        .fields
        .iter()
        .find(|candidate| candidate.name.as_deref() == Some(field))
        .ok_or_else(|| NativeEmitError(format!("unknown named payload field `{field}`")))
}
