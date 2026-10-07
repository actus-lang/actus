use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData};
use cranelift_frontend::FunctionBuilder;

use crate::ast::{CaseBranch, Pattern, VariantPayload};

use super::super::enum_layout::{EnumLayout, EnumVariantLayout};
use super::super::layout::LayoutRegistry;
use super::super::native::{FunctionRef, NativeEmitError};
use super::super::structs::copy_bytes;
use super::super::types::NativeType;

pub(super) type BranchLocals<'a> =
    (HashMap<&'a String, cranelift_codegen::ir::Value>, HashMap<&'a String, NativeType>);

pub(super) fn add_payload_types<'a>(
    branch: &'a crate::ast::CaseBranch,
    subject_type: NativeType,
    types: &mut HashMap<&'a String, NativeType>,
    layouts: &LayoutRegistry,
) {
    let crate::ast::Pattern::Variant { enum_name: _, variant, payload, .. } = &branch.pattern
    else {
        return;
    };
    let NativeType::Enum(enum_id) = subject_type else { return };
    let Some(variant_layout) = layouts.enum_variant(enum_id, variant) else { return };
    match payload {
        crate::ast::VariantPayload::Positional(items) => {
            add_positional_payload_types(items, variant_layout, types)
        }
        crate::ast::VariantPayload::Named(items) => {
            add_named_payload_types(items, variant_layout, types)
        }
        crate::ast::VariantPayload::Unit => {}
    }
}

fn add_positional_payload_types<'a>(
    items: &'a [crate::ast::PatternBinding],
    variant_layout: &super::super::enum_layout::EnumVariantLayout,
    types: &mut HashMap<&'a String, NativeType>,
) {
    for (index, item) in items.iter().enumerate() {
        let Some(field) = variant_layout.fields.get(index) else { continue };
        insert_payload_type(types, &item.name, field.ty);
    }
}

fn add_named_payload_types<'a>(
    items: &'a [crate::ast::NamedPattern],
    variant_layout: &super::super::enum_layout::EnumVariantLayout,
    types: &mut HashMap<&'a String, NativeType>,
) {
    for item in items {
        let Some(field) = variant_layout
            .fields
            .iter()
            .find(|field| field.name.as_deref() == Some(item.name.as_str()))
        else {
            continue;
        };
        insert_payload_type(types, &item.binding.name, field.ty);
    }
}

fn insert_payload_type<'a>(
    types: &mut HashMap<&'a String, NativeType>,
    binding: &'a String,
    ty: NativeType,
) {
    if binding != "_" {
        types.insert(binding, ty);
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn bind_payload<'a>(
    function: &mut FunctionBuilder<'_>,
    subject: cranelift_codegen::ir::Value,
    subject_type: NativeType,
    branch: &'a CaseBranch,
    locals: &HashMap<&'a String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&'a String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<BranchLocals<'a>, NativeEmitError> {
    let mut branch_locals = locals.clone();
    let mut branch_types = local_types.clone();
    bind_variant_case(
        function,
        subject,
        subject_type,
        branch,
        &mut branch_locals,
        &mut branch_types,
        functions,
        layouts,
    )?;
    Ok((branch_locals, branch_types))
}

#[allow(clippy::too_many_arguments)]
fn bind_variant_case<'a>(
    function: &mut FunctionBuilder<'_>,
    subject: cranelift_codegen::ir::Value,
    subject_type: NativeType,
    branch: &'a CaseBranch,
    locals: &mut HashMap<&'a String, cranelift_codegen::ir::Value>,
    types: &mut HashMap<&'a String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    let Pattern::Variant { variant, payload, .. } = &branch.pattern else { return Ok(()) };
    let NativeType::Enum(enum_id) = subject_type else { return Ok(()) };
    let enum_layout = layouts
        .enum_layout(enum_id)
        .ok_or_else(|| NativeEmitError("missing enum layout".to_owned()))?;
    let variant_layout = layouts
        .enum_variant(enum_id, variant)
        .ok_or_else(|| NativeEmitError("missing case variant layout".to_owned()))?;
    bind_variant_payload(
        function,
        subject,
        enum_layout,
        payload_bindings(payload),
        branch,
        variant_layout,
        locals,
        types,
        functions,
        layouts,
    )
}

fn payload_bindings(payload: &VariantPayload) -> Vec<(Option<&str>, &String)> {
    match payload {
        VariantPayload::Positional(items) => {
            items.iter().map(|item| (None, &item.name)).collect::<Vec<_>>()
        }
        VariantPayload::Named(items) => items
            .iter()
            .map(|item| (Some(item.name.as_str()), &item.binding.name))
            .collect::<Vec<_>>(),
        VariantPayload::Unit => Vec::new(),
    }
}

#[allow(clippy::too_many_arguments)]
fn bind_variant_payload<'a>(
    function: &mut FunctionBuilder<'_>,
    subject: cranelift_codegen::ir::Value,
    enum_layout: &EnumLayout,
    bindings: Vec<(Option<&str>, &'a String)>,
    branch: &'a CaseBranch,
    variant_layout: &EnumVariantLayout,
    locals: &mut HashMap<&'a String, cranelift_codegen::ir::Value>,
    types: &mut HashMap<&'a String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    if enum_layout.niche_pointer && branch_variant(branch) == Some("Some") {
        return bind_niche_payload(branch, bindings, subject, variant_layout, locals, types);
    }
    bind_regular_payload(
        function,
        subject,
        enum_layout.payload_offset,
        bindings,
        branch,
        variant_layout,
        locals,
        types,
        functions,
        layouts,
    )
}

fn branch_variant(branch: &CaseBranch) -> Option<&str> {
    let Pattern::Variant { variant, .. } = &branch.pattern else { return None };
    Some(variant.as_str())
}

fn bind_niche_payload<'a>(
    branch: &'a CaseBranch,
    bindings: Vec<(Option<&str>, &'a String)>,
    subject: cranelift_codegen::ir::Value,
    variant_layout: &EnumVariantLayout,
    locals: &mut HashMap<&'a String, cranelift_codegen::ir::Value>,
    types: &mut HashMap<&'a String, NativeType>,
) -> Result<(), NativeEmitError> {
    for (index, (_, binding)) in bindings.into_iter().enumerate() {
        if binding == "_" {
            continue;
        }
        let field = variant_layout
            .fields
            .get(index)
            .ok_or_else(|| NativeEmitError("missing niche payload field layout".to_owned()))?;
        let key = branch_binding(branch, binding)
            .ok_or_else(|| NativeEmitError("missing case binding".to_owned()))?;
        locals.insert(key, subject);
        types.insert(key, field.ty);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn bind_regular_payload<'a>(
    function: &mut FunctionBuilder<'_>,
    subject: cranelift_codegen::ir::Value,
    payload_offset: u32,
    bindings: Vec<(Option<&str>, &'a String)>,
    branch: &'a CaseBranch,
    variant_layout: &EnumVariantLayout,
    locals: &mut HashMap<&'a String, cranelift_codegen::ir::Value>,
    types: &mut HashMap<&'a String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    for (index, (name, binding)) in bindings.into_iter().enumerate() {
        if binding != "_" {
            load_payload_binding(
                function,
                subject,
                payload_offset,
                index,
                name,
                binding,
                branch,
                variant_layout,
                locals,
                types,
                functions,
                layouts,
            )?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn load_payload_binding<'a>(
    function: &mut FunctionBuilder<'_>,
    subject: cranelift_codegen::ir::Value,
    payload_offset: u32,
    index: usize,
    name: Option<&str>,
    binding: &str,
    branch: &'a CaseBranch,
    variant_layout: &EnumVariantLayout,
    branch_locals: &mut HashMap<&'a String, cranelift_codegen::ir::Value>,
    branch_types: &mut HashMap<&'a String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    let field = name
        .and_then(|field_name| {
            variant_layout.fields.iter().find(|field| field.name.as_deref() == Some(field_name))
        })
        .or_else(|| variant_layout.fields.get(index))
        .ok_or_else(|| NativeEmitError("missing case payload field layout".to_owned()))?;
    let address = function.ins().iadd_imm_s(subject, i64::from(payload_offset + field.offset));
    let value = match field.ty {
        NativeType::Struct(_) | NativeType::Array(_) => address,
        NativeType::Pack(_) if layouts.is_inline_pack(field.ty) => address,
        NativeType::Enum(id) if !layouts.is_niche_option(id) => {
            let layout = layouts
                .enum_layout(id)
                .ok_or_else(|| NativeEmitError("missing enum payload layout".to_owned()))?;
            let allocator = functions.get("__actus_enum_allocate").ok_or_else(|| {
                NativeEmitError("enum payload allocator is unavailable".to_owned())
            })?;
            let size = function.ins().iconst(layouts.pointer_type, i64::from(layout.size));
            let allocation = function.ins().call(allocator.reference, &[size]);
            let destination =
                function.inst_results(allocation).first().copied().ok_or_else(|| {
                    NativeEmitError("enum payload allocation returned no pointer".to_owned())
                })?;
            copy_bytes(function, address, destination, layout.size);
            destination
        }
        _ => function.ins().load(layouts.ir_type(field.ty)?, MemFlagsData::new(), address, 0),
    };
    let key = branch_binding(branch, binding)
        .ok_or_else(|| NativeEmitError("missing case binding".to_owned()))?;
    branch_locals.insert(key, value);
    branch_types.insert(key, field.ty);
    Ok(())
}

fn branch_binding<'a>(branch: &'a CaseBranch, name: &str) -> Option<&'a String> {
    let Pattern::Variant { payload, .. } = &branch.pattern else { return None };
    match payload {
        VariantPayload::Positional(items) => {
            items.iter().find(|item| item.name == name).map(|item| &item.name)
        }
        VariantPayload::Named(items) => {
            items.iter().find(|item| item.binding.name == name).map(|item| &item.binding.name)
        }
        VariantPayload::Unit => None,
    }
}
