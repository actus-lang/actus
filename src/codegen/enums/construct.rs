use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData, types};
use cranelift_frontend::FunctionBuilder;

use crate::ast::{Argument, Expr};

use super::super::calls::CallLoweringContext;
use super::super::enum_layout::{EnumFieldLayout, EnumLayout, EnumVariantLayout};
use super::super::expressions::lower_expression;
use super::super::layout::LayoutRegistry;
use super::super::native::{FunctionRef, NativeEmitError};
use super::super::structs::copy_bytes;
use super::super::types::NativeType;

#[allow(clippy::too_many_arguments)]
pub(crate) fn lower_enum_constructor(
    function: &mut FunctionBuilder<'_>,
    receiver: &Expr,
    variant: &str,
    arguments: &[Argument],
    context: &CallLoweringContext<'_, '_>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let (enum_layout, variant_layout) = constructor_layouts(receiver, variant, context.layouts)?;
    if enum_layout.niche_pointer {
        return lower_niche_option_constructor(function, variant, arguments, context);
    }
    lower_regular_enum_constructor(function, arguments, enum_layout, variant_layout, context)
}

fn constructor_layouts<'a>(
    receiver: &Expr,
    variant: &str,
    layouts: &'a LayoutRegistry,
) -> Result<(&'a EnumLayout, &'a EnumVariantLayout), NativeEmitError> {
    let enum_name = super::types::enum_receiver_name(receiver)
        .ok_or_else(|| NativeEmitError("enum constructor requires a type receiver".to_owned()))?;
    let (enum_id, variant_layout) =
        layouts.enum_constructor(enum_name, variant).ok_or_else(|| {
            NativeEmitError(format!("unknown enum constructor `{enum_name}.{variant}`"))
        })?;
    let enum_layout = layouts
        .enum_layout(enum_id)
        .ok_or_else(|| NativeEmitError(format!("missing enum layout `{enum_id}`")))?;
    Ok((enum_layout, variant_layout))
}

#[allow(clippy::too_many_arguments)]
fn lower_regular_enum_constructor(
    function: &mut FunctionBuilder<'_>,
    arguments: &[Argument],
    enum_layout: &EnumLayout,
    variant_layout: &EnumVariantLayout,
    context: &CallLoweringContext<'_, '_>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let address = allocate_enum(function, context.functions, context.layouts, enum_layout.size)?;
    let discriminant = function.ins().iconst(types::I32, i64::from(variant_layout.discriminant));
    function.ins().store(
        MemFlagsData::new(),
        discriminant,
        address,
        enum_layout.discriminant_offset as i32,
    );
    lower_enum_payload(
        function,
        address,
        arguments,
        enum_layout.payload_offset,
        variant_layout,
        context,
    )?;
    Ok(address)
}

fn allocate_enum(
    function: &mut FunctionBuilder<'_>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
    size: u32,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let allocator = functions
        .get("__actus_enum_allocate")
        .ok_or_else(|| NativeEmitError("native enum allocator is unavailable".to_owned()))?;
    let size_value = function.ins().iconst(layouts.pointer_type, i64::from(size));
    let allocation = function.ins().call(allocator.reference, &[size_value]);
    function
        .inst_results(allocation)
        .first()
        .copied()
        .ok_or_else(|| NativeEmitError("native enum allocator returned no value".to_owned()))
}

#[allow(clippy::too_many_arguments)]
fn lower_enum_payload(
    function: &mut FunctionBuilder<'_>,
    address: cranelift_codegen::ir::Value,
    arguments: &[Argument],
    payload_offset: u32,
    variant_layout: &EnumVariantLayout,
    context: &CallLoweringContext<'_, '_>,
) -> Result<(), NativeEmitError> {
    for (field, argument) in ordered_arguments(&variant_layout.fields, arguments)? {
        let value = lower_expression(
            function,
            &argument.expression,
            context.locals,
            context.local_types,
            context.functions,
            context.cleanup_schedule,
            context.string_data,
            context.layouts,
        )?;
        let destination =
            function.ins().iadd_imm_s(address, i64::from(payload_offset + field.offset));
        store_payload(function, destination, value, field.ty, context.layouts)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn lower_niche_option_constructor(
    function: &mut FunctionBuilder<'_>,
    variant: &str,
    arguments: &[Argument],
    context: &CallLoweringContext<'_, '_>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    match variant {
        "None" if arguments.is_empty() => {
            Ok(function.ins().iconst(context.layouts.pointer_type, 0))
        }
        "Some" if arguments.len() == 1 => lower_expression(
            function,
            &arguments[0].expression,
            context.locals,
            context.local_types,
            context.functions,
            context.cleanup_schedule,
            context.string_data,
            context.layouts,
        ),
        _ => Err(NativeEmitError(format!("invalid niche Option constructor `{variant}`"))),
    }
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
    if matches!(ty, NativeType::Void) {
        return Ok(());
    }
    if let NativeType::Enum(id) = ty
        && layouts.is_niche_option(id)
    {
        function.ins().store(MemFlagsData::new(), value, destination, 0);
    } else if matches!(ty, NativeType::Struct(_) | NativeType::Enum(_) | NativeType::Array(_))
        || layouts.is_inline_pack(ty)
    {
        let size = layouts
            .type_size(ty)
            .ok_or_else(|| NativeEmitError("missing enum payload layout".to_owned()))?;
        copy_bytes(function, value, destination, size, layouts)?;
    } else {
        function.ins().store(MemFlagsData::new(), value, destination, 0);
    }
    Ok(())
}
