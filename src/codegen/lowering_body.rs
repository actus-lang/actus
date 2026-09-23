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
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let mut locals = initial_locals.clone();
    let mut types = initial_types.clone();
    match super::lowering_statements::lower_statements(
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
        Flow::Return(value) => Ok(value),
        Flow::Fallthrough => {
            Err(NativeEmitError("native function requires a return value".to_owned()))
        }
        Flow::Break | Flow::Continue => {
            Err(NativeEmitError("loop control escaped its loop during native lowering".to_owned()))
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn lower_case_block<'source>(
    function: &mut FunctionBuilder<'_>,
    block: &'source crate::ast::Block,
    locals: &HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    let mut branch_locals = locals.clone();
    let mut branch_types = types.clone();
    let flow = super::lowering_statements::lower_statements(
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
    if matches!(flow, Flow::Fallthrough) {
        emit_scope_cleanup(
            function,
            cleanup_schedule,
            block,
            &branch_locals,
            &branch_types,
            functions,
            layouts,
        )?;
    }
    Ok(flow)
}
