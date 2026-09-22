use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData};
use cranelift_frontend::FunctionBuilder;

use crate::ast::{Expr, StructFieldInit};

use super::expressions::emit_buffer_drop;
use super::expressions::lower_expression;
use super::layout::LayoutRegistry;
use super::literals::StringDataValues;
use super::native::{FunctionRef, NativeEmitError};
use super::types::NativeType;

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_struct_literal(
    function: &mut FunctionBuilder<'_>,
    name: &str,
    fields: &[StructFieldInit],
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let id = layouts
        .id_for(name)
        .ok_or_else(|| NativeEmitError(format!("missing layout for struct `{name}`")))?;
    let layout =
        layouts.get(id).ok_or_else(|| NativeEmitError(format!("missing layout `{id}`")))?;
    let slot = function.func.create_sized_stack_slot(layouts.stack_slot(layout));
    let address = function.ins().stack_addr(layouts.pointer_type, slot, 0);
    for field in fields {
        let field_layout = layout
            .fields
            .iter()
            .find(|candidate| candidate.name == field.name)
            .ok_or_else(|| NativeEmitError(format!("unknown native field `{}`", field.name)))?;
        let value = lower_expression(
            function,
            &field.value,
            locals,
            local_types,
            functions,
            string_data,
            layouts,
        )?;
        if matches!(field_layout.ty, NativeType::Struct(_) | NativeType::Enum(_)) {
            let size = layouts
                .type_size(field_layout.ty)
                .ok_or_else(|| NativeEmitError("missing nested field layout".to_owned()))?;
            let destination = function.ins().iadd_imm_s(address, i64::from(field_layout.offset));
            copy_bytes(function, value, destination, size);
        } else {
            function.ins().store(MemFlagsData::new(), value, address, field_layout.offset as i32);
        }
    }
    Ok(address)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_field_access(
    function: &mut FunctionBuilder<'_>,
    object: &Expr,
    field: &str,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
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
        string_data,
        layouts,
    )?;
    if matches!(field_layout.ty, NativeType::Struct(_)) {
        return Ok(function.ins().iadd_imm_s(address, i64::from(field_layout.offset)));
    }
    Ok(function.ins().load(
        field_layout.ty.ir_type(layouts.pointer_type),
        MemFlagsData::new(),
        address,
        field_layout.offset as i32,
    ))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_field_assignment(
    function: &mut FunctionBuilder<'_>,
    object: &Expr,
    field: &str,
    value: &Expr,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    let (address, field_layout) = lower_field_address(
        function,
        object,
        field,
        locals,
        local_types,
        functions,
        string_data,
        layouts,
    )?;
    let value =
        lower_expression(function, value, locals, local_types, functions, string_data, layouts)?;
    if matches!(field_layout.ty, NativeType::Struct(_) | NativeType::Enum(_)) {
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
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<(cranelift_codegen::ir::Value, super::layout::FieldLayout), NativeEmitError> {
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
    let address =
        lower_expression(function, object, locals, local_types, functions, string_data, layouts)?;
    Ok((address, field_layout))
}

fn copy_bytes(
    function: &mut FunctionBuilder<'_>,
    source: cranelift_codegen::ir::Value,
    destination: cranelift_codegen::ir::Value,
    size: u32,
) {
    for offset in 0..size {
        let source_address = function.ins().iadd_imm_s(source, i64::from(offset));
        let destination_address = function.ins().iadd_imm_s(destination, i64::from(offset));
        let byte = function.ins().load(
            cranelift_codegen::ir::types::I8,
            MemFlagsData::new(),
            source_address,
            0,
        );
        function.ins().store(MemFlagsData::new(), byte, destination_address, 0);
    }
}

pub(super) fn emit_struct_drop(
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
    for field in layout
        .fields
        .iter()
        .rev()
        .filter(|field| field.owned && !moved_fields.iter().any(|moved| moved == &field.name))
    {
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
            NativeType::Struct(nested_id) => {
                emit_struct_drop_except(
                    function,
                    field_address,
                    nested_id,
                    &[],
                    functions,
                    layouts,
                )?;
            }
            NativeType::Enum(_) | NativeType::Int | NativeType::String => {}
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn emit_partial_binding_drop(
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
pub(super) fn emit_binding_drop(
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
    emit_struct_drop(function, address, id, functions, layouts)
}

pub(super) fn expression_native_type(
    expression: &Expr,
    local_types: &HashMap<&String, NativeType>,
    layouts: &LayoutRegistry,
) -> Option<NativeType> {
    match expression {
        Expr::Identifier { name, .. } => local_types.get(name).copied(),
        Expr::StructLit { name, .. } => layouts.type_for_name(name),
        Expr::Grouping { expression, .. } | Expr::Borrow { expression, .. } => {
            expression_native_type(expression, local_types, layouts)
        }
        Expr::FieldAccess { object, field, .. } => {
            expression_native_type(object, local_types, layouts)
                .and_then(|ty| field_type(ty, field, layouts))
        }
        Expr::MethodCall { .. } => super::enums::enum_expression_type(expression, layouts),
        Expr::Integer { .. } => Some(NativeType::Int),
        Expr::StringLiteral { .. } => Some(NativeType::String),
        _ => None,
    }
}

pub(super) fn field_type(
    ty: NativeType,
    field: &str,
    layouts: &LayoutRegistry,
) -> Option<NativeType> {
    let NativeType::Struct(id) = ty else { return None };
    layouts.get(id)?.fields.iter().find(|candidate| candidate.name == field).map(|field| field.ty)
}
