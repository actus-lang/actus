use std::collections::HashMap;

use cranelift_frontend::FunctionBuilder;

use crate::ast::Stmt;

use super::super::cleanup::{CleanupContext, emit_scope_cleanup};
use super::super::literals::StringDataValues;
use super::super::native::{BackendEmitError, FunctionRef};
use super::super::types::NativeType;
use super::{Flow, LoweredCleanupSchedule};
use crate::codegen::cranelift::layout::LayoutRegistry;

type CaseBlockState<'source> = (
    Flow,
    HashMap<&'source String, cranelift_codegen::ir::Value>,
    HashMap<&'source String, NativeType>,
    Option<cranelift_codegen::ir::Value>,
);

#[allow(clippy::too_many_arguments)]
pub(crate) fn lower_body(
    function: &mut FunctionBuilder<'_>,
    statements: &[Stmt],
    initial_locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    initial_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &LoweredCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, BackendEmitError> {
    let mut locals = initial_locals.clone();
    let mut types = initial_types.clone();
    match super::statements::lower_statements(
        function,
        statements,
        &mut locals,
        &mut types,
        functions,
        None,
        cleanup_schedule,
        string_data,
        layouts,
    )? {
        flow @ (Flow::Return(_, _) | Flow::VoidReturn | Flow::Fallthrough) => Ok(flow),
        Flow::Break | Flow::Continue => {
            Err(BackendEmitError("loop control escaped its loop during native lowering".to_owned()))
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn lower_case_block<'source>(
    function: &mut FunctionBuilder<'_>,
    block: &'source crate::ast::Block,
    cleanup_span: crate::lexer::SourceSpan,
    locals: &HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &LoweredCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
    targets: Option<super::LoopTargets>,
    value_producing: bool,
) -> Result<CaseBlockState<'source>, BackendEmitError> {
    let mut branch_locals = locals.clone();
    let mut branch_types = types.clone();
    let (prefix, tail) = case_block_parts(block, value_producing);
    let flow = super::statements::lower_statements(
        function,
        prefix,
        &mut branch_locals,
        &mut branch_types,
        functions,
        targets.clone(),
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    let tail_value = lower_case_tail(
        function,
        flow,
        tail,
        &branch_locals,
        &branch_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
        targets,
    )?;
    let flow = finish_case_block(
        function,
        flow,
        cleanup_span,
        &branch_locals,
        &branch_types,
        functions,
        cleanup_schedule,
        layouts,
    )?;
    Ok((flow, branch_locals, branch_types, tail_value))
}

fn case_block_parts(
    block: &crate::ast::Block,
    value_producing: bool,
) -> (&[Stmt], Option<&crate::ast::Expr>) {
    if !value_producing {
        return (block.statements.as_slice(), None);
    }
    match block.statements.split_last() {
        Some((Stmt::Expression { expression, .. }, prefix)) => (prefix, Some(expression)),
        _ => (block.statements.as_slice(), None),
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_case_tail(
    function: &mut FunctionBuilder<'_>,
    flow: Flow,
    tail: Option<&crate::ast::Expr>,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &LoweredCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
    targets: Option<super::LoopTargets>,
) -> Result<Option<cranelift_codegen::ir::Value>, BackendEmitError> {
    if !matches!(flow, Flow::Fallthrough) {
        return Ok(None);
    }
    tail.map(|expression| {
        super::super::expressions::lower_expression_with_targets(
            function,
            expression,
            locals,
            types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
            targets,
        )
    })
    .transpose()
}

#[allow(clippy::too_many_arguments)]
fn finish_case_block(
    function: &mut FunctionBuilder<'_>,
    flow: Flow,
    cleanup_span: crate::lexer::SourceSpan,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &LoweredCleanupSchedule,
    layouts: &LayoutRegistry,
) -> Result<Flow, BackendEmitError> {
    if matches!(flow, Flow::Fallthrough) {
        let mut cleanup = CleanupContext::new(function, locals, types, functions, layouts);
        emit_scope_cleanup(&mut cleanup, cleanup_schedule, cleanup_span)?;
    }
    Ok(flow)
}
