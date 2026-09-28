use std::collections::HashMap;

use cranelift_frontend::FunctionBuilder;

use crate::ast::Stmt;

use super::super::cleanup::emit_scope_cleanup;
use super::super::layout::LayoutRegistry;
use super::super::literals::StringDataValues;
use super::super::native::{FunctionRef, NativeEmitError};
use super::super::types::NativeType;
use super::{Flow, NativeCleanupSchedule};

#[allow(clippy::too_many_arguments)]
pub(crate) fn lower_body(
    function: &mut FunctionBuilder<'_>,
    statements: &[Stmt],
    initial_locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    initial_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
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
        flow @ (Flow::Return(_) | Flow::VoidReturn | Flow::Fallthrough) => Ok(flow),
        Flow::Break | Flow::Continue => {
            Err(NativeEmitError("loop control escaped its loop during native lowering".to_owned()))
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
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    let mut branch_locals = locals.clone();
    let mut branch_types = types.clone();
    let flow = super::statements::lower_statements(
        function,
        &block.statements,
        &mut branch_locals,
        &mut branch_types,
        functions,
        None,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    finish_case_block(
        function,
        flow,
        cleanup_span,
        &branch_locals,
        &branch_types,
        functions,
        cleanup_schedule,
        layouts,
    )
}

#[allow(clippy::too_many_arguments)]
fn finish_case_block(
    function: &mut FunctionBuilder<'_>,
    flow: Flow,
    cleanup_span: crate::lexer::SourceSpan,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    if matches!(flow, Flow::Fallthrough) {
        emit_scope_cleanup(
            function,
            cleanup_schedule,
            cleanup_span,
            locals,
            types,
            functions,
            layouts,
        )?;
    }
    Ok(flow)
}
