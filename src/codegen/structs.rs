use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData};
use cranelift_frontend::FunctionBuilder;

use crate::ast::{Expr, StructFieldInit};

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
        if matches!(field_layout.ty, NativeType::Struct(_)) {
            return Err(NativeEmitError("nested struct stores are not lowered yet".to_owned()));
        }
        let value = lower_expression(
            function,
            &field.value,
            locals,
            local_types,
            functions,
            string_data,
            layouts,
        )?;
        function.ins().store(MemFlagsData::new(), value, address, field_layout.offset as i32);
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
        .ok_or_else(|| NativeEmitError(format!("unknown native field `{field}`")))?;
    let address =
        lower_expression(function, object, locals, local_types, functions, string_data, layouts)?;
    if matches!(field_layout.ty, NativeType::Struct(_)) {
        return Err(NativeEmitError("nested struct field loads are not lowered yet".to_owned()));
    }
    Ok(function.ins().load(
        field_layout.ty.ir_type(layouts.pointer_type),
        MemFlagsData::new(),
        address,
        field_layout.offset as i32,
    ))
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
