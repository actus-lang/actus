use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData, types};
use cranelift_frontend::FunctionBuilder;

use crate::ast::{Argument, Expr};

use super::enum_layout::EnumFieldLayout;
use super::expressions::lower_expression;
use super::layout::LayoutRegistry;
use super::literals::StringDataValues;
use super::native::{FunctionRef, NativeEmitError};
use super::types::NativeType;

pub(super) fn enum_receiver_name(receiver: &Expr) -> Option<&str> {
    let Expr::Identifier { name, .. } = receiver else { return None };
    Some(name)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_enum_constructor(
    function: &mut FunctionBuilder<'_>,
    receiver: &Expr,
    variant: &str,
    arguments: &[Argument],
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let enum_name = enum_receiver_name(receiver)
        .ok_or_else(|| NativeEmitError("enum constructor requires a type receiver".to_owned()))?;
    let (enum_id, variant_layout) =
        layouts.enum_constructor(enum_name, variant).ok_or_else(|| {
            NativeEmitError(format!("unknown enum constructor `{enum_name}.{variant}`"))
        })?;
    let enum_layout = layouts
        .enum_layout(enum_id)
        .ok_or_else(|| NativeEmitError(format!("missing enum layout `{enum_id}`")))?;
    let slot = function.func.create_sized_stack_slot(layouts.enum_stack_slot(enum_layout));
    let address = function.ins().stack_addr(layouts.pointer_type, slot, 0);
    let discriminant = function.ins().iconst(types::I32, i64::from(variant_layout.discriminant));
    function.ins().store(
        MemFlagsData::new(),
        discriminant,
        address,
        enum_layout.discriminant_offset as i32,
    );
    for (field, argument) in ordered_arguments(variant_layout.fields.as_slice(), arguments)? {
        let value = lower_expression(
            function,
            &argument.expression,
            locals,
            local_types,
            functions,
            string_data,
            layouts,
        )?;
        let destination = function
            .ins()
            .iadd_imm_s(address, i64::from(enum_layout.payload_offset + field.offset));
        store_payload(function, destination, value, field.ty, layouts)?;
    }
    Ok(address)
}

fn ordered_arguments<'a>(
    fields: &'a [EnumFieldLayout],
    arguments: &'a [Argument],
) -> Result<Vec<(&'a EnumFieldLayout, &'a Argument)>, NativeEmitError> {
    if arguments.iter().all(|argument| argument.name.is_none()) {
        return fields.iter().zip(arguments).map(Ok).collect();
    }
    fields
        .iter()
        .map(|field| {
            let name = field.name.as_deref().ok_or_else(|| {
                NativeEmitError("tuple enum constructors require positional arguments".to_owned())
            })?;
            let argument = arguments
                .iter()
                .find(|argument| argument.name.as_deref() == Some(name))
                .ok_or_else(|| NativeEmitError(format!("missing enum payload `{name}`")))?;
            Ok((field, argument))
        })
        .collect()
}

fn store_payload(
    function: &mut FunctionBuilder<'_>,
    destination: cranelift_codegen::ir::Value,
    value: cranelift_codegen::ir::Value,
    ty: NativeType,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    if let NativeType::Struct(id) | NativeType::Enum(id) = ty {
        let size = layouts
            .type_size(ty)
            .ok_or_else(|| NativeEmitError(format!("missing payload layout `{id}`")))?;
        copy_bytes(function, value, destination, size);
    } else {
        function.ins().store(MemFlagsData::new(), value, destination, 0);
    }
    Ok(())
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
        let byte = function.ins().load(types::I8, MemFlagsData::new(), source_address, 0);
        function.ins().store(MemFlagsData::new(), byte, destination_address, 0);
    }
}

pub(super) fn enum_expression_type(
    expression: &Expr,
    layouts: &LayoutRegistry,
) -> Option<NativeType> {
    let (receiver, variant) = match expression {
        Expr::MethodCall { receiver, method, .. } => (receiver.as_ref(), method.as_str()),
        Expr::FieldAccess { object, field, .. } => (object.as_ref(), field.as_str()),
        _ => return None,
    };
    let enum_name = enum_receiver_name(receiver)?;
    let (id, _) = layouts.enum_constructor(enum_name, variant)?;
    Some(NativeType::Enum(id))
}
