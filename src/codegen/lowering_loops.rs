use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, types};
use cranelift_frontend::FunctionBuilder;

use crate::ast::Block;

use super::super::control_flow::{assigned_outer_bindings, carried_values, jump_with_values};
use super::super::layout::LayoutRegistry;
use super::super::literals::StringDataValues;
use super::super::native::{FunctionRef, NativeEmitError};
use super::super::types::NativeType;
use super::{Flow, LoopTargets, NativeCleanupSchedule};

pub(super) fn emit_loop_jump(
    function: &mut FunctionBuilder<'_>,
    targets: Option<LoopTargets>,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    continue_loop: bool,
) -> Result<Flow, NativeEmitError> {
    let Some(targets) = targets else {
        let keyword = if continue_loop { "continue" } else { "break" };
        return Err(NativeEmitError(format!("{keyword} has no native loop target")));
    };
    let target = if continue_loop { targets.header } else { targets.exit };
    let values = carried_values(locals, &targets.carried)?;
    jump_with_values(function, target, values);
    Ok(if continue_loop { Flow::Continue } else { Flow::Break })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_loop<'source>(
    function: &mut FunctionBuilder<'_>,
    block: &'source Block,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &mut HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    let carried = assigned_outer_bindings(&block.statements, locals);
    let header = function.create_block();
    let body = function.create_block();
    let exit = function.create_block();
    for _ in &carried {
        function.append_block_param(header, types::I32);
        function.append_block_param(exit, types::I32);
    }
    let initial_values = carried_values(locals, &carried)?;
    jump_with_values(function, header, initial_values);
    function.switch_to_block(header);
    function.ins().jump(body, &[]);
    function.seal_block(body);
    function.switch_to_block(body);
    let mut loop_locals = locals.clone();
    let mut loop_types = types.clone();
    let header_values = function.block_params(header).to_vec();
    for (name, value) in carried.iter().zip(header_values) {
        let binding = locals.keys().find(|binding| binding.as_str() == name).unwrap();
        loop_locals.insert(binding, value);
    }
    let flow = super::lowering_statements::lower_statements(
        function,
        &block.statements,
        &mut loop_locals,
        &mut loop_types,
        functions,
        Some(LoopTargets { header, exit, carried: carried.clone() }),
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    if matches!(flow, Flow::Fallthrough) {
        let values = carried_values(&loop_locals, &carried)?;
        jump_with_values(function, header, values);
    }
    function.seal_block(header);
    if matches!(flow, Flow::Break | Flow::Fallthrough | Flow::Continue) {
        function.switch_to_block(exit);
        function.seal_block(exit);
        let exit_values = function.block_params(exit).to_vec();
        for (name, value) in carried.iter().zip(exit_values) {
            let binding = locals.keys().find(|binding| binding.as_str() == name).unwrap();
            locals.insert(binding, value);
        }
        Ok(Flow::Fallthrough)
    } else {
        Ok(flow)
    }
}
