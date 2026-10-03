use std::collections::HashMap;

use cranelift_codegen::ir::InstBuilder;
use cranelift_frontend::FunctionBuilder;

use crate::ast::Block;

use super::super::control_flow::{assigned_outer_bindings, carried_values, jump_with_values};
use super::super::layout::LayoutRegistry;
use super::super::literals::StringDataValues;
use super::super::native::{FunctionRef, NativeEmitError};
use super::super::types::NativeType;
use super::{Flow, LoopTargets, NativeCleanupSchedule};

type LoopState<'source> =
    (HashMap<&'source String, cranelift_codegen::ir::Value>, HashMap<&'source String, NativeType>);

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
    let (mut loop_locals, mut loop_types) =
        initialize_loop(function, locals, types, layouts, &carried, header, body, exit)?;
    let flow = super::statements::lower_statements(
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
    finish_loop(function, locals, &loop_locals, &carried, header, exit, flow)
}

#[allow(clippy::too_many_arguments)]
fn initialize_loop<'source>(
    function: &mut FunctionBuilder<'_>,
    locals: &HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &HashMap<&'source String, NativeType>,
    layouts: &LayoutRegistry,
    carried: &[String],
    header: cranelift_codegen::ir::Block,
    body: cranelift_codegen::ir::Block,
    exit: cranelift_codegen::ir::Block,
) -> Result<LoopState<'source>, NativeEmitError> {
    append_loop_parameters(function, carried, types, layouts, header, exit)?;
    let initial_values = carried_values(locals, carried)?;
    jump_with_values(function, header, initial_values);
    function.switch_to_block(header);
    function.ins().jump(body, &[]);
    function.seal_block(body);
    function.switch_to_block(body);
    let mut loop_locals = locals.clone();
    let loop_types = types.clone();
    let header_values = function.block_params(header).to_vec();
    for (name, value) in carried.iter().zip(header_values) {
        let binding = find_loop_binding(locals, name)?;
        loop_locals.insert(binding, value);
    }
    Ok((loop_locals, loop_types))
}

fn finish_loop<'source>(
    function: &mut FunctionBuilder<'_>,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    loop_locals: &HashMap<&'source String, cranelift_codegen::ir::Value>,
    carried: &[String],
    header: cranelift_codegen::ir::Block,
    exit: cranelift_codegen::ir::Block,
    flow: Flow,
) -> Result<Flow, NativeEmitError> {
    if matches!(flow, Flow::Fallthrough) {
        let values = carried_values(loop_locals, carried)?;
        jump_with_values(function, header, values);
    }
    function.seal_block(header);
    if !matches!(flow, Flow::Break | Flow::Fallthrough | Flow::Continue) {
        return Ok(flow);
    }
    function.switch_to_block(exit);
    function.seal_block(exit);
    let exit_values = function.block_params(exit).to_vec();
    for (name, value) in carried.iter().zip(exit_values) {
        let binding = find_loop_binding(locals, name)?;
        locals.insert(binding, value);
    }
    Ok(Flow::Fallthrough)
}

fn find_loop_binding<'source>(
    locals: &HashMap<&'source String, cranelift_codegen::ir::Value>,
    name: &str,
) -> Result<&'source String, NativeEmitError> {
    locals
        .keys()
        .find(|binding| binding.as_str() == name)
        .copied()
        .ok_or_else(|| NativeEmitError(format!("loop binding `{name}` is unavailable")))
}

fn append_loop_parameters(
    function: &mut FunctionBuilder<'_>,
    carried: &[String],
    types: &HashMap<&String, NativeType>,
    layouts: &LayoutRegistry,
    header: cranelift_codegen::ir::Block,
    exit: cranelift_codegen::ir::Block,
) -> Result<(), NativeEmitError> {
    for name in carried {
        let native_type = types
            .iter()
            .find(|(binding, _)| binding.as_str() == name)
            .map(|(_, native_type)| *native_type)
            .ok_or_else(|| NativeEmitError(format!("loop binding `{name}` has no native type")))?;
        let ir_type = layouts.ir_type(native_type)?;
        function.append_block_param(header, ir_type);
        function.append_block_param(exit, ir_type);
    }
    Ok(())
}
