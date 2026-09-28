use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;

use crate::ast::{CaseBody, Expr, Pattern};

use super::case_payload::{BranchLocals, add_payload_types, bind_payload};
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
    let merge = function.create_block();
    let subject_type = initializer_type(subject, local_types, functions, layouts)?;
    let result_type = branch_type(branches, subject_type, local_types, functions, layouts)?;
    function.append_block_param(merge, layouts.ir_type(result_type));
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
            let fallback = function.ins().iconst(layouts.ir_type(result_type), 0);
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
            following,
            merge,
            result_type,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        )?;
        if let Some(branch_value) = branch_value {
            let argument = cranelift_codegen::ir::BlockArg::Value(branch_value);
            function.ins().jump(merge, [&argument]);
        }
        if branch.guard.is_none() {
            function.seal_block(matched);
        }
        if following != merge {
            function.switch_to_block(following);
            function.seal_block(following);
        }
    }
    Ok(())
}

fn branch_type(
    branches: &[crate::ast::CaseBranch],
    subject_type: NativeType,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<NativeType, NativeEmitError> {
    let mut inferred = None;
    for branch in branches {
        let mut branch_types = local_types.clone();
        add_payload_types(branch, subject_type, &mut branch_types, layouts);
        let candidate = match &branch.body {
            CaseBody::Expression(expression) => {
                Some(initializer_type(expression, &branch_types, functions, layouts)?)
            }
            CaseBody::Block(_) => None,
        };
        if let Some(candidate) = candidate {
            if let Some(expected) = inferred {
                if expected != candidate {
                    return Err(NativeEmitError(
                        "case branches have different native types".to_owned(),
                    ));
                }
            } else {
                inferred = Some(candidate);
            }
        }
    }
    Ok(inferred.unwrap_or(NativeType::Void))
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
                crate::ast::LiteralPattern::Integer(value) => parse_integer_pattern(value)?,
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
            let variant_layout = layouts.enum_variant(enum_id, variant).ok_or_else(|| {
                NativeEmitError(format!("unknown case variant `{enum_name}.{variant}`"))
            })?;
            if enum_layout.niche_pointer {
                let is_none = variant == "None";
                return Ok(function.ins().icmp_imm_s(
                    if is_none { IntCC::Equal } else { IntCC::NotEqual },
                    subject,
                    0,
                ));
            }
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

fn parse_integer_pattern(value: &str) -> Result<i64, NativeEmitError> {
    let (negative, digits) =
        value.strip_prefix('-').map_or((false, value), |digits| (true, digits));
    let magnitude = digits
        .strip_prefix("0x")
        .or_else(|| digits.strip_prefix("0X"))
        .map_or_else(|| digits.parse::<i128>(), |hex| i128::from_str_radix(hex, 16))
        .map_err(|_| NativeEmitError(format!("invalid integer pattern `{value}`")))?;
    let signed = if negative { magnitude.checked_neg() } else { Some(magnitude) }
        .ok_or_else(|| NativeEmitError(format!("integer pattern `{value}` overflows")))?;
    i64::try_from(signed)
        .map_err(|_| NativeEmitError(format!("integer pattern `{value}` overflows")))
}

#[allow(clippy::too_many_arguments)]
fn lower_case_branch<'a>(
    function: &mut FunctionBuilder<'_>,
    subject: cranelift_codegen::ir::Value,
    subject_type: NativeType,
    branch: &'a crate::ast::CaseBranch,
    following: cranelift_codegen::ir::Block,
    merge: cranelift_codegen::ir::Block,
    result_type: NativeType,
    locals: &HashMap<&'a String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&'a String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &super::model::NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Option<cranelift_codegen::ir::Value>, NativeEmitError> {
    let branch_locals =
        bind_payload(function, subject, subject_type, branch, locals, local_types, layouts)?;
    if let Some(guard) = &branch.guard {
        let condition = lower_expression(
            function,
            guard,
            &branch_locals.0,
            &branch_locals.1,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        )?;
        let body = function.create_block();
        emit_guard_branch(function, condition, body, following, merge, result_type, layouts);
        function.seal_block(function.current_block().expect("guard block is active"));
        function.switch_to_block(body);
        let branch_value = lower_case_body(
            function,
            branch,
            &branch_locals,
            functions,
            result_type,
            cleanup_schedule,
            string_data,
            layouts,
        )?;
        function.seal_block(body);
        return Ok(branch_value);
    }
    lower_case_body(
        function,
        branch,
        &branch_locals,
        functions,
        result_type,
        cleanup_schedule,
        string_data,
        layouts,
    )
}

fn emit_guard_branch(
    function: &mut FunctionBuilder<'_>,
    condition: cranelift_codegen::ir::Value,
    body: cranelift_codegen::ir::Block,
    following: cranelift_codegen::ir::Block,
    merge: cranelift_codegen::ir::Block,
    result_type: NativeType,
    layouts: &LayoutRegistry,
) {
    if following == merge {
        let fallback = function.ins().iconst(layouts.ir_type(result_type), 0);
        let fallback_arg = cranelift_codegen::ir::BlockArg::Value(fallback);
        function.ins().brif(condition, body, &[], following, [&fallback_arg]);
    } else {
        function.ins().brif(condition, body, &[], following, &[]);
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_case_body<'a>(
    function: &mut FunctionBuilder<'_>,
    branch: &'a crate::ast::CaseBranch,
    branch_locals: &BranchLocals<'a>,
    functions: &HashMap<String, FunctionRef>,
    result_type: NativeType,
    cleanup_schedule: &super::model::NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Option<cranelift_codegen::ir::Value>, NativeEmitError> {
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
            branch.span,
            &branch_locals.0,
            &branch_locals.1,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        )? {
            Flow::Return(value) => {
                function.ins().return_(&[value]);
                return Ok(None);
            }
            Flow::Fallthrough => function.ins().iconst(layouts.ir_type(result_type), 0),
            Flow::VoidReturn => {
                function.ins().return_(&[]);
                return Ok(None);
            }
            Flow::Break | Flow::Continue => {
                return Err(NativeEmitError("loop control escaped case block".to_owned()));
            }
        },
    };
    Ok(Some(branch_value))
}
