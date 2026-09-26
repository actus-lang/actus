use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData};
use cranelift_frontend::FunctionBuilder;

use super::super::expressions::emit_buffer_drop;
use super::super::layout::LayoutRegistry;
use super::super::native::{FunctionRef, NativeEmitError};
use super::super::types::NativeType;

pub(crate) fn emit_struct_drop(
    function: &mut FunctionBuilder<'_>,
    address: cranelift_codegen::ir::Value,
    id: usize,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    emit_struct_drop_except(function, address, id, &[], functions, layouts)
}

fn emit_struct_drop_except(
    function: &mut FunctionBuilder<'_>,
    address: cranelift_codegen::ir::Value,
    id: usize,
    moved_fields: &[String],
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    let layout =
        layouts.get(id).ok_or_else(|| NativeEmitError(format!("missing drop layout `{id}`")))?;
    for field in layout.fields.iter().rev().filter(|field| field.owned) {
        let nested_paths = moved_field_suffixes(moved_fields, &field.name);
        if nested_paths.iter().any(String::is_empty) {
            continue;
        }
        let field_address = function.ins().iadd_imm_s(address, i64::from(field.offset));
        match field.ty {
            NativeType::Buffer => {
                let handle = function.ins().load(
                    layouts.pointer_type,
                    MemFlagsData::new(),
                    field_address,
                    0,
                );
                let target = functions.get("actus_buffer_drop").ok_or_else(|| {
                    NativeEmitError(
                        "native runtime function `actus_buffer_drop` is unavailable".to_owned(),
                    )
                })?;
                function.ins().call(target.reference, &[handle]);
            }
            NativeType::Struct(nested_id) => emit_struct_drop_except(
                function,
                field_address,
                nested_id,
                &nested_paths,
                functions,
                layouts,
            )?,
            NativeType::Enum(_) | NativeType::Int | NativeType::String | NativeType::FatPointer => {
            }
        }
    }
    Ok(())
}

fn moved_field_suffixes(moved_fields: &[String], field_name: &str) -> Vec<String> {
    moved_fields
        .iter()
        .filter_map(|moved| {
            moved.strip_prefix(field_name).and_then(|suffix| {
                suffix
                    .strip_prefix('.')
                    .map(str::to_owned)
                    .or_else(|| (suffix.is_empty()).then(String::new))
            })
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_partial_binding_drop(
    function: &mut FunctionBuilder<'_>,
    name: &str,
    moved_fields: &[String],
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    let Some(NativeType::Struct(id)) =
        types.iter().find(|(binding, _)| binding.as_str() == name).map(|(_, ty)| *ty)
    else {
        return Ok(());
    };
    let address = locals
        .iter()
        .find(|(binding, _)| binding.as_str() == name)
        .map(|(_, value)| *value)
        .ok_or_else(|| NativeEmitError(format!("native binding `{name}` is unavailable")))?;
    emit_struct_drop_except(function, address, id, moved_fields, functions, layouts)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_binding_drop(
    function: &mut FunctionBuilder<'_>,
    name: &str,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    if types.iter().any(|(binding, ty)| binding.as_str() == name && *ty == NativeType::Buffer) {
        return emit_buffer_drop(function, name, locals, functions);
    }
    let Some(binding_type) =
        types.iter().find(|(binding, _)| binding.as_str() == name).map(|(_, ty)| *ty)
    else {
        return Ok(());
    };
    let address = locals
        .iter()
        .find(|(binding, _)| binding.as_str() == name)
        .map(|(_, value)| *value)
        .ok_or_else(|| NativeEmitError(format!("native binding `{name}` is unavailable")))?;
    match binding_type {
        NativeType::Struct(id) => emit_struct_drop(function, address, id, functions, layouts),
        NativeType::Enum(_) => {
            let size = layouts
                .type_size(binding_type)
                .ok_or_else(|| NativeEmitError("missing enum drop layout".to_owned()))?;
            let target = functions
                .get("actus_enum_drop")
                .ok_or_else(|| NativeEmitError("native enum drop is unavailable".to_owned()))?;
            let size = function.ins().iconst(layouts.pointer_type, i64::from(size));
            function.ins().call(target.reference, &[address, size]);
            Ok(())
        }
        _ => Ok(()),
    }
}
