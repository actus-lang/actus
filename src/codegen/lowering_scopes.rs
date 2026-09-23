use std::collections::HashMap;

use cranelift_frontend::FunctionBuilder;

use crate::ast::Block;

use super::super::cleanup::emit_scope_cleanup;
use super::super::layout::LayoutRegistry;
use super::super::literals::StringDataValues;
use super::super::native::{FunctionRef, NativeEmitError};
use super::super::types::NativeType;
use super::{Flow, LoopTargets, NativeCleanupSchedule};

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_scoped_block<'source>(
    function: &mut FunctionBuilder<'_>,
    block: &'source Block,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &mut HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    targets: Option<LoopTargets>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    let mut nested_locals = locals.clone();
    let mut nested_types = types.clone();
    let flow = super::lowering_statements::lower_statements(
        function,
        &block.statements,
        &mut nested_locals,
        &mut nested_types,
        functions,
        targets,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    if matches!(flow, Flow::Fallthrough) {
        emit_scope_cleanup(
            function,
            cleanup_schedule,
            block,
            &nested_locals,
            &nested_types,
            functions,
            layouts,
        )?;
    }
    Ok(flow)
}
