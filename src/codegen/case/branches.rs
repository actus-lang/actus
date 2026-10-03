use cranelift_codegen::ir::{InstBuilder, TrapCode, types};
use cranelift_frontend::FunctionBuilder;

use crate::ast::CaseBody;

use super::super::expressions::lower_expression;
use super::super::lowering::{Flow, lower_case_block};
use super::super::native::NativeEmitError;
use super::super::types::NativeType;
use super::context::CaseLoweringContext;
use super::matching::match_pattern;
use super::payload::{BranchLocals, bind_payload};

const CASE_EXHAUSTIVENESS_TRAP: TrapCode = TrapCode::unwrap_user(1);

pub(super) fn emit_case_branches(
    function: &mut FunctionBuilder<'_>,
    branches: &[crate::ast::CaseBranch],
    subject_value: cranelift_codegen::ir::Value,
    subject_type: NativeType,
    result_type: NativeType,
    merge: cranelift_codegen::ir::Block,
    context: &CaseLoweringContext<'_, '_>,
) -> Result<(), NativeEmitError> {
    let next_blocks =
        (0..branches.len().saturating_sub(1)).map(|_| function.create_block()).collect::<Vec<_>>();
    for (index, branch) in branches.iter().enumerate() {
        let following = next_blocks.get(index).copied().unwrap_or(merge);
        emit_case_branch(
            function,
            branch,
            following,
            merge,
            subject_value,
            subject_type,
            result_type,
            context,
        )?;
        if following != merge {
            function.switch_to_block(following);
            function.seal_block(following);
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn emit_case_branch(
    function: &mut FunctionBuilder<'_>,
    branch: &crate::ast::CaseBranch,
    following: cranelift_codegen::ir::Block,
    merge: cranelift_codegen::ir::Block,
    subject_value: cranelift_codegen::ir::Value,
    subject_type: NativeType,
    result_type: NativeType,
    context: &CaseLoweringContext<'_, '_>,
) -> Result<(), NativeEmitError> {
    let matched = function.create_block();
    let condition =
        match_pattern(function, subject_value, subject_type, &branch.pattern, context.layouts)?;
    branch_condition(function, condition, matched, following, merge);
    function.switch_to_block(matched);
    let branch_value = lower_case_branch(
        function,
        subject_value,
        subject_type,
        branch,
        following,
        merge,
        result_type,
        context,
    )?;
    finish_case_branch(function, branch_value, branch, matched, merge);
    Ok(())
}

fn finish_case_branch(
    function: &mut FunctionBuilder<'_>,
    branch_value: Option<cranelift_codegen::ir::Value>,
    branch: &crate::ast::CaseBranch,
    matched: cranelift_codegen::ir::Block,
    merge: cranelift_codegen::ir::Block,
) {
    if let Some(branch_value) = branch_value {
        let argument = cranelift_codegen::ir::BlockArg::Value(branch_value);
        function.ins().jump(merge, [&argument]);
    }
    if branch.guard.is_none() {
        function.seal_block(matched);
    }
}

fn branch_condition(
    function: &mut FunctionBuilder<'_>,
    condition: cranelift_codegen::ir::Value,
    matched: cranelift_codegen::ir::Block,
    following: cranelift_codegen::ir::Block,
    merge: cranelift_codegen::ir::Block,
) {
    if following == merge {
        let failure = function.create_block();
        function.ins().brif(condition, matched, &[], failure, &[]);
        emit_case_exhaustiveness_trap(function, failure);
    } else {
        function.ins().brif(condition, matched, &[], following, &[]);
    }
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
    context: &CaseLoweringContext<'_, 'a>,
) -> Result<Option<cranelift_codegen::ir::Value>, NativeEmitError> {
    let branch_locals = bind_branch_payload(function, subject, subject_type, branch, context)?;
    if let Some(guard) = &branch.guard {
        return lower_guarded_case_branch(
            function,
            guard,
            branch,
            &branch_locals,
            following,
            merge,
            result_type,
            context,
        );
    }
    lower_case_body(function, branch, &branch_locals, result_type, context)
}

fn bind_branch_payload<'a>(
    function: &mut FunctionBuilder<'_>,
    subject: cranelift_codegen::ir::Value,
    subject_type: NativeType,
    branch: &'a crate::ast::CaseBranch,
    context: &CaseLoweringContext<'_, 'a>,
) -> Result<BranchLocals<'a>, NativeEmitError> {
    bind_payload(
        function,
        subject,
        subject_type,
        branch,
        context.locals,
        context.local_types,
        context.layouts,
    )
}

#[allow(clippy::too_many_arguments)]
fn lower_guarded_case_branch<'a>(
    function: &mut FunctionBuilder<'_>,
    guard: &crate::ast::Expr,
    branch: &'a crate::ast::CaseBranch,
    branch_locals: &BranchLocals<'a>,
    following: cranelift_codegen::ir::Block,
    merge: cranelift_codegen::ir::Block,
    result_type: NativeType,
    context: &CaseLoweringContext<'_, 'a>,
) -> Result<Option<cranelift_codegen::ir::Value>, NativeEmitError> {
    let condition = lower_expression(
        function,
        guard,
        &branch_locals.0,
        &branch_locals.1,
        context.functions,
        context.cleanup_schedule,
        context.string_data,
        context.layouts,
    )?;
    let body = function.create_block();
    emit_guard_branch(function, condition, body, following, merge)?;
    function.seal_block(function.current_block().expect("guard block is active"));
    function.switch_to_block(body);
    let branch_value = lower_case_body(function, branch, branch_locals, result_type, context)?;
    function.seal_block(body);
    Ok(branch_value)
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

fn lower_case_body<'a>(
    function: &mut FunctionBuilder<'_>,
    branch: &'a crate::ast::CaseBranch,
    branch_locals: &BranchLocals<'a>,
    result_type: NativeType,
    context: &CaseLoweringContext<'_, 'a>,
) -> Result<Option<cranelift_codegen::ir::Value>, NativeEmitError> {
    let branch_value = match &branch.body {
        CaseBody::Expression(expression) => {
            Some(lower_case_expression(function, expression, branch_locals, context)?)
        }
        CaseBody::Block(block) => {
            lower_case_block_body(function, block, branch, branch_locals, result_type, context)?
        }
    };
    Ok(branch_value)
}

fn lower_case_expression<'a>(
    function: &mut FunctionBuilder<'_>,
    expression: &crate::ast::Expr,
    branch_locals: &BranchLocals<'a>,
    context: &CaseLoweringContext<'_, 'a>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    lower_expression(
        function,
        expression,
        &branch_locals.0,
        &branch_locals.1,
        context.functions,
        context.cleanup_schedule,
        context.string_data,
        context.layouts,
    )
}

fn lower_case_block_body<'a>(
    function: &mut FunctionBuilder<'_>,
    block: &crate::ast::Block,
    branch: &'a crate::ast::CaseBranch,
    branch_locals: &BranchLocals<'a>,
    result_type: NativeType,
    context: &CaseLoweringContext<'_, 'a>,
) -> Result<Option<cranelift_codegen::ir::Value>, NativeEmitError> {
    let flow = lower_case_block(
        function,
        block,
        branch.span,
        &branch_locals.0,
        &branch_locals.1,
        context.functions,
        context.cleanup_schedule,
        context.string_data,
        context.layouts,
        context.loop_targets.clone(),
    )?;
    finish_case_flow(function, flow, result_type)
}

fn finish_case_flow(
    function: &mut FunctionBuilder<'_>,
    flow: Flow,
    result_type: NativeType,
) -> Result<Option<cranelift_codegen::ir::Value>, NativeEmitError> {
    match flow {
        Flow::Return(value, _) => {
            function.ins().return_(&[value]);
            Ok(None)
        }
        Flow::Fallthrough if matches!(result_type, NativeType::Void) => {
            Ok(Some(function.ins().iconst(types::I8, 0)))
        }
        Flow::Fallthrough => Err(NativeEmitError(
            "case block must return a value in a value-producing case".to_owned(),
        )),
        Flow::VoidReturn => {
            function.ins().return_(&[]);
            Ok(None)
        }
        Flow::Break | Flow::Continue => Ok(None),
    }
}
