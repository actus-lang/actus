use std::collections::HashMap;

use cranelift_frontend::FunctionBuilder;

use crate::ast::Stmt;

use super::super::super::layout::LayoutRegistry;
use super::super::super::literals::StringDataValues;
use super::super::super::native::{FunctionRef, NativeEmitError};
use super::super::super::types::NativeType;
use super::super::{Flow, LoopTargets, NativeCleanupSchedule};

#[allow(clippy::too_many_arguments)]
#[rustfmt::skip]
pub(super) fn lower_structured_control_statement<'source>(
    function: &mut FunctionBuilder<'_>,
    statement: &'source Stmt,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &mut HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    targets: Option<LoopTargets>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    match statement {
        Stmt::Block(block) => super::super::scopes::lower_scoped_block(function, block, locals, types, functions, targets, cleanup_schedule, string_data, layouts),
        Stmt::Loop(block) => super::super::loops::lower_loop(function, block, locals, types, functions, cleanup_schedule, string_data, layouts),
        Stmt::ForRange { binding, start, end, body, .. } => super::super::loops::lower_for_range(function, binding, start, end, body, locals, types, functions, cleanup_schedule, string_data, layouts),
        Stmt::ForArray { binding, collection, body, .. } => super::super::loops::lower_for_array(function, binding, collection, body, locals, types, functions, cleanup_schedule, string_data, layouts),
        _ => unreachable!("structured control helper received a non-structured statement"),
    }
}
