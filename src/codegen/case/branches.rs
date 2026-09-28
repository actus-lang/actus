use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, TrapCode, types};
use cranelift_frontend::FunctionBuilder;

use crate::ast::CaseBody;

use super::super::case_payload::{BranchLocals, bind_payload};
use super::super::expressions::lower_expression;
use super::super::layout::LayoutRegistry;
use super::super::literals::StringDataValues;
use super::super::lowering::{Flow, lower_case_block};
use super::super::native::{FunctionRef, NativeEmitError};
use super::super::types::NativeType;
use super::matching::match_pattern;

const CASE_EXHAUSTIVENESS_TRAP: TrapCode = TrapCode::unwrap_user(1);

#[allow(clippy::too_many_arguments)]
pub(super) fn emit_case_branches(
    function: &mut FunctionBuilder<'_>,
    branches: &[crate::ast::CaseBranch],
    subject_value: cranelift_codegen::ir::Value,
    subject_type: NativeType,
    result_type: NativeType,
    merge: cranelift_codegen::ir::Block,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &super::super::model::NativeCleanupSchedule,
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
            let failure = function.create_block();
            function.ins().brif(condition, matched, &[], failure, &[]);
            emit_case_exhaustiveness_trap(function, failure);
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
    cleanup_schedule: &super::super::model::NativeCleanupSchedule,
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
        emit_guard_branch(function, condition, body, following, merge)?;
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
) -> Result<(), NativeEmitError> {
    if following == merge {
        let failure = function.create_block();
        function.ins().brif(condition, body, &[], failure, &[]);
        emit_case_exhaustiveness_trap(function, failure);
    } else {
        function.ins().brif(condition, body, &[], following, &[]);
    }
    Ok(())
}

fn emit_case_exhaustiveness_trap(
    function: &mut FunctionBuilder<'_>,
    failure: cranelift_codegen::ir::Block,
) {
    function.switch_to_block(failure);
    function.ins().trap(CASE_EXHAUSTIVENESS_TRAP);
    function.seal_block(failure);
}

#[allow(clippy::too_many_arguments)]
fn lower_case_body<'a>(
    function: &mut FunctionBuilder<'_>,
    branch: &'a crate::ast::CaseBranch,
    branch_locals: &BranchLocals<'a>,
    functions: &HashMap<String, FunctionRef>,
    result_type: NativeType,
    cleanup_schedule: &super::super::model::NativeCleanupSchedule,
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
            Flow::Fallthrough if matches!(result_type, NativeType::Void) => {
                function.ins().iconst(types::I8, 0)
            }
            Flow::Fallthrough => {
                return Err(NativeEmitError(
                    "case block must return a value in a value-producing case".to_owned(),
                ));
            }
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
