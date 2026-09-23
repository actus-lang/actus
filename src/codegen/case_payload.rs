use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData};
use cranelift_frontend::FunctionBuilder;

use crate::ast::{CaseBranch, Pattern, VariantPayload};

use super::enum_layout::EnumVariantLayout;
use super::layout::LayoutRegistry;
use super::native::NativeEmitError;
use super::types::NativeType;

pub(super) type BranchLocals<'a> =
    (HashMap<&'a String, cranelift_codegen::ir::Value>, HashMap<&'a String, NativeType>);

#[allow(clippy::too_many_arguments)]
pub(super) fn bind_payload<'a>(
    function: &mut FunctionBuilder<'_>,
    subject: cranelift_codegen::ir::Value,
    subject_type: NativeType,
    branch: &'a CaseBranch,
    locals: &HashMap<&'a String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&'a String, NativeType>,
    layouts: &LayoutRegistry,
) -> Result<BranchLocals<'a>, NativeEmitError> {
    let mut branch_locals = locals.clone();
    let mut branch_types = local_types.clone();
    let Pattern::Variant { variant, payload, .. } = &branch.pattern else {
        return Ok((branch_locals, branch_types));
    };
    let NativeType::Enum(enum_id) = subject_type else {
        return Ok((branch_locals, branch_types));
    };
    let enum_layout = layouts
        .enum_layout(enum_id)
        .ok_or_else(|| NativeEmitError("missing enum layout".to_owned()))?;
    let variant_layout = layouts
        .enum_variant(enum_id, variant)
        .ok_or_else(|| NativeEmitError("missing case variant layout".to_owned()))?;
    let bindings = match payload {
        VariantPayload::Positional(items) => {
            items.iter().map(|item| (None, &item.name)).collect::<Vec<_>>()
        }
        VariantPayload::Named(items) => items
            .iter()
            .map(|item| (Some(item.name.as_str()), &item.binding.name))
            .collect::<Vec<_>>(),
        VariantPayload::Unit => Vec::new(),
    };
    for (index, (name, binding)) in bindings.into_iter().enumerate() {
        if binding != "_" {
            load_payload_binding(
                function,
                subject,
                enum_layout.payload_offset,
                index,
                name,
                binding,
                branch,
                variant_layout,
                &mut branch_locals,
                &mut branch_types,
                layouts,
            )?;
        }
    }
    Ok((branch_locals, branch_types))
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
        NativeType::Struct(_) | NativeType::Enum(_) => address,
        _ => function.ins().load(
            field.ty.ir_type(layouts.pointer_type),
            MemFlagsData::new(),
            address,
            0,
        ),
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
