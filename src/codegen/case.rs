use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;

use crate::ast::{CaseBody, Expr, Pattern, VariantPayload};

use super::expressions::{initializer_type, lower_expression};
use super::layout::LayoutRegistry;
use super::literals::StringDataValues;
use super::lowering::{Flow, lower_case_block};
use super::native::{FunctionRef, NativeEmitError};
use super::types::NativeType;

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_case(
    function: &mut FunctionBuilder<'_>,
    subject: &Expr,
    branches: &[crate::ast::CaseBranch],
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &super::model::NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let subject_value = lower_expression(
        function,
        subject,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    let result_type = branch_type(branches, local_types, functions, layouts);
    let merge = function.create_block();
    function.append_block_param(merge, result_type.ir_type(layouts.pointer_type));
    let subject_type = initializer_type(subject, local_types, functions, layouts);
    emit_case_branches(
        function,
        branches,
        subject_value,
        subject_type,
        result_type,
        merge,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    function.switch_to_block(merge);
    function.seal_block(merge);
    Ok(function.block_params(merge)[0])
}

#[allow(clippy::too_many_arguments)]
fn emit_case_branches(
    function: &mut FunctionBuilder<'_>,
    branches: &[crate::ast::CaseBranch],
    subject_value: cranelift_codegen::ir::Value,
    subject_type: NativeType,
    result_type: NativeType,
    merge: cranelift_codegen::ir::Block,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &super::model::NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<(), NativeEmitError> {
    let next_blocks =
        (0..branches.len().saturating_sub(1)).map(|_| function.create_block()).collect::<Vec<_>>();
    for (index, branch) in branches.iter().enumerate() {
        let matched = function.create_block();
        let following = next_blocks.get(index).copied().unwrap_or(merge);
        let condition =
            match_pattern(function, subject_value, subject_type, &branch.pattern, layouts)?;
        if following == merge {
            let fallback = function.ins().iconst(result_type.ir_type(layouts.pointer_type), 0);
            let fallback_arg = cranelift_codegen::ir::BlockArg::Value(fallback);
            function.ins().brif(condition, matched, &[], following, [&fallback_arg]);
        } else {
            function.ins().brif(condition, matched, &[], following, &[]);
        }
        function.switch_to_block(matched);
        let branch_value = lower_case_branch(
            function,
            subject_value,
            subject_type,
            branch,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        )?;
        let argument = cranelift_codegen::ir::BlockArg::Value(branch_value);
        function.ins().jump(merge, [&argument]);
        function.seal_block(matched);
        if following != merge {
            function.switch_to_block(following);
            function.seal_block(following);
        }
    }
    function.switch_to_block(merge);
    function.seal_block(merge);
    Ok(())
}

fn branch_type(
    branches: &[crate::ast::CaseBranch],
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> NativeType {
    branches
        .iter()
        .find_map(|branch| match &branch.body {
            CaseBody::Expression(expression) => {
                Some(initializer_type(expression, local_types, functions, layouts))
            }
            CaseBody::Block(_) => None,
        })
        .unwrap_or(NativeType::Int)
}

fn match_pattern(
    function: &mut FunctionBuilder<'_>,
    subject: cranelift_codegen::ir::Value,
    subject_type: NativeType,
    pattern: &Pattern,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    match pattern {
        Pattern::Wildcard { .. } => Ok(function.ins().iconst(types::I8, 1)),
        Pattern::Literal { value, .. } => {
            let expected = match value {
                crate::ast::LiteralPattern::Integer(value) => value.parse().unwrap_or_default(),
                crate::ast::LiteralPattern::Bool(value) => i64::from(*value),
            };
            Ok(function.ins().icmp_imm_s(IntCC::Equal, subject, expected))
        }
        Pattern::Variant { enum_name, variant, .. } => {
            let NativeType::Enum(enum_id) = subject_type else {
                return Err(NativeEmitError(format!("case subject is not enum `{enum_name}`")));
            };
            let enum_layout = layouts.enum_layout(enum_id).ok_or_else(|| {
                NativeEmitError("missing enum layout for case subject".to_owned())
            })?;
            let (_, variant_layout) =
                layouts.enum_constructor(enum_name, variant).ok_or_else(|| {
                    NativeEmitError(format!("unknown case variant `{enum_name}.{variant}`"))
                })?;
            let discriminant = function.ins().load(
                types::I32,
                MemFlagsData::new(),
                subject,
                enum_layout.discriminant_offset as i32,
            );
            Ok(function.ins().icmp_imm_s(
                IntCC::Equal,
                discriminant,
                i64::from(variant_layout.discriminant),
            ))
        }
    }
}

type BranchLocals<'a> =
    (HashMap<&'a String, cranelift_codegen::ir::Value>, HashMap<&'a String, NativeType>);

#[allow(clippy::too_many_arguments)]
fn lower_case_branch<'a>(
    function: &mut FunctionBuilder<'_>,
    subject: cranelift_codegen::ir::Value,
    subject_type: NativeType,
    branch: &'a crate::ast::CaseBranch,
    locals: &HashMap<&'a String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&'a String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &super::model::NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let branch_locals =
        bind_payload(function, subject, subject_type, branch, locals, local_types, layouts)?;
    let branch_value = match &branch.body {
        CaseBody::Expression(expression) => lower_expression(
            function,
            expression,
            &branch_locals.0,
            &branch_locals.1,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        )?,
        CaseBody::Block(block) => match lower_case_block(
            function,
            block,
            &branch_locals.0,
            &branch_locals.1,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        )? {
            Flow::Return(value) => value,
            Flow::Fallthrough => {
                return Err(NativeEmitError(
                    "case block must produce a value with return".to_owned(),
                ));
            }
            Flow::Break | Flow::Continue => {
                return Err(NativeEmitError("loop control escaped case block".to_owned()));
            }
        },
    };
    Ok(branch_value)
}

fn bind_payload<'a>(
    function: &mut FunctionBuilder<'_>,
    subject: cranelift_codegen::ir::Value,
    subject_type: NativeType,
    branch: &'a crate::ast::CaseBranch,
    locals: &HashMap<&'a String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&'a String, NativeType>,
    layouts: &LayoutRegistry,
) -> Result<BranchLocals<'a>, NativeEmitError> {
    let mut branch_locals = locals.clone();
    let mut branch_types = local_types.clone();
    let Pattern::Variant { enum_name, variant, payload, .. } = &branch.pattern else {
        return Ok((branch_locals, branch_types));
    };
    let NativeType::Enum(enum_id) = subject_type else { return Ok((branch_locals, branch_types)) };
    let enum_layout = layouts
        .enum_layout(enum_id)
        .ok_or_else(|| NativeEmitError("missing enum layout".to_owned()))?;
    let (_, variant_layout) = layouts
        .enum_constructor(enum_name, variant)
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
    branch: &'a crate::ast::CaseBranch,
    variant_layout: &super::enum_layout::EnumVariantLayout,
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

fn branch_binding<'a>(branch: &'a crate::ast::CaseBranch, name: &str) -> Option<&'a String> {
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
