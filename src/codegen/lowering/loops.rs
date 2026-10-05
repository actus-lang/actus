use std::collections::HashMap;

use cranelift_codegen::ir::{BlockArg, InstBuilder, condcodes::IntCC};
use cranelift_frontend::FunctionBuilder;

use crate::ast::{Block, ForBinding};

use super::super::control_flow::{assigned_outer_bindings, carried_values, jump_with_values};
use super::super::expressions::{initializer_type, lower_expression};
use super::super::layout::LayoutRegistry;
use super::super::literals::StringDataValues;
use super::super::native::{FunctionRef, NativeEmitError};
use super::super::types::NativeType;
use super::{Flow, LoopTargets, NativeCleanupSchedule};

type LoopState<'source> =
    (HashMap<&'source String, cranelift_codegen::ir::Value>, HashMap<&'source String, NativeType>);

struct ForRangeState {
    condition: cranelift_codegen::ir::Block,
    body: cranelift_codegen::ir::Block,
    increment: cranelift_codegen::ir::Block,
    exit: cranelift_codegen::ir::Block,
    carried: Vec<String>,
    carried_outer: Vec<String>,
    condition_values: Vec<cranelift_codegen::ir::Value>,
    index_position: usize,
    index_value: cranelift_codegen::ir::Value,
}

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
#[rustfmt::skip]
pub(super) fn lower_for_range<'source>(
    function: &mut FunctionBuilder<'_>,
    binding: &'source ForBinding,
    start: &crate::ast::Expr,
    end: &crate::ast::Expr,
    block: &'source Block,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &mut HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    let index_type =
        NativeType::from_name(binding.ty.as_deref().unwrap_or("u32")).ok_or_else(|| {
            NativeEmitError("for range binding has no native integer type".to_owned())
        })?;
    let start_value = lower_for_bound(function, start, locals, types, functions, cleanup_schedule, string_data, layouts)?;
    let end_value = lower_for_bound(function, end, locals, types, functions, cleanup_schedule, string_data, layouts)?;
    let carried_outer = assigned_outer_bindings(&block.statements, locals);
    let mut carried = vec![binding.name.clone()];
    carried.extend(carried_outer.iter().cloned());
    carried.sort();
    carried.dedup();
    types.insert(&binding.name, index_type);
    let state = initialize_for_range(function, binding, index_type, start_value, end_value, carried, carried_outer, locals, types, layouts)?;
    let flow = lower_for_range_body(function, binding, index_type, block, state, locals, types, functions, cleanup_schedule, string_data, layouts)?;
    types.remove(&binding.name);
    Ok(flow)
}

#[allow(clippy::too_many_arguments)]
fn initialize_for_range<'source>(
    function: &mut FunctionBuilder<'_>,
    binding: &'source ForBinding,
    index_type: NativeType,
    start_value: cranelift_codegen::ir::Value,
    end_value: cranelift_codegen::ir::Value,
    carried: Vec<String>,
    carried_outer: Vec<String>,
    locals: &HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    layouts: &LayoutRegistry,
) -> Result<ForRangeState, NativeEmitError> {
    let (condition, body, increment, exit) =
        create_for_range_blocks(function, &carried, types, layouts)?;
    let initial_values = initial_for_range_values(&binding.name, start_value, &carried, locals)?;
    jump_with_values(function, condition, initial_values);
    function.switch_to_block(condition);
    let condition_values = function.block_params(condition).to_vec();
    let index_position =
        carried.iter().position(|name| name == &binding.name).expect("range binding is carried");
    let index_value = condition_values[index_position];
    let condition_code = if matches!(index_type, NativeType::Integer { signed: true, .. }) {
        IntCC::SignedLessThan
    } else {
        IntCC::UnsignedLessThan
    };
    let comparison = function.ins().icmp(condition_code, index_value, end_value);
    let branch_values = condition_values.iter().copied().map(BlockArg::Value).collect::<Vec<_>>();
    function.ins().brif(comparison, body, &[], exit, &branch_values);
    function.seal_block(body);
    Ok(ForRangeState {
        condition,
        body,
        increment,
        exit,
        carried,
        carried_outer,
        condition_values,
        index_position,
        index_value,
    })
}

fn create_for_range_blocks(
    function: &mut FunctionBuilder<'_>,
    carried: &[String],
    types: &HashMap<&String, NativeType>,
    layouts: &LayoutRegistry,
) -> Result<
    (
        cranelift_codegen::ir::Block,
        cranelift_codegen::ir::Block,
        cranelift_codegen::ir::Block,
        cranelift_codegen::ir::Block,
    ),
    NativeEmitError,
> {
    let condition = function.create_block();
    let body = function.create_block();
    let increment = function.create_block();
    let exit = function.create_block();
    append_loop_parameters(function, carried, types, layouts, condition, exit)?;
    for name in carried {
        let native_type = types
            .iter()
            .find(|(candidate, _)| candidate.as_str() == name)
            .map(|(_, native_type)| *native_type)
            .ok_or_else(|| NativeEmitError(format!("for binding `{name}` has no native type")))?;
        function.append_block_param(increment, layouts.ir_type(native_type)?);
    }
    Ok((condition, body, increment, exit))
}

fn initial_for_range_values(
    binding_name: &str,
    start_value: cranelift_codegen::ir::Value,
    carried: &[String],
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
) -> Result<Vec<cranelift_codegen::ir::Value>, NativeEmitError> {
    let mut initial_values = Vec::with_capacity(carried.len());
    for name in carried {
        if name == binding_name {
            initial_values.push(start_value);
        } else {
            initial_values.extend(carried_values(locals, std::slice::from_ref(name))?);
        }
    }
    Ok(initial_values)
}

#[allow(clippy::too_many_arguments)]
fn lower_for_range_body<'source>(
    function: &mut FunctionBuilder<'_>,
    binding: &'source ForBinding,
    index_type: NativeType,
    block: &'source Block,
    state: ForRangeState,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &mut HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    function.switch_to_block(state.body);
    let mut body_locals = locals.clone();
    let mut body_types = types.clone();
    body_locals.insert(&binding.name, state.index_value);
    body_types.insert(&binding.name, index_type);
    for (position, name) in state.carried.iter().enumerate() {
        if name != &binding.name
            && let Some(binding_name) = locals.keys().find(|candidate| candidate.as_str() == name)
        {
            body_locals.insert(binding_name, state.condition_values[position]);
        }
    }
    let flow = super::statements::lower_statements(
        function,
        &block.statements,
        &mut body_locals,
        &mut body_types,
        functions,
        Some(LoopTargets {
            header: state.increment,
            exit: state.exit,
            carried: state.carried.clone(),
        }),
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    if matches!(flow, Flow::Fallthrough) {
        jump_with_values(function, state.increment, carried_values(&body_locals, &state.carried)?);
    }
    function.switch_to_block(state.increment);
    let increment_values = function.block_params(state.increment).to_vec();
    let next_index = function.ins().iadd_imm_u(increment_values[state.index_position], 1);
    let mut next_values = increment_values;
    next_values[state.index_position] = next_index;
    jump_with_values(function, state.condition, next_values);
    function.seal_block(state.increment);
    function.seal_block(state.condition);
    function.switch_to_block(state.exit);
    function.seal_block(state.exit);
    let exit_values = function.block_params(state.exit).to_vec();
    carry_for_range_bindings(locals, &state.carried, state.carried_outer, &exit_values);
    Ok(Flow::Fallthrough)
}

#[allow(clippy::too_many_arguments)]
fn lower_for_bound(
    function: &mut FunctionBuilder<'_>,
    expression: &crate::ast::Expr,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    lower_expression(
        function,
        expression,
        locals,
        types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )
}

fn carry_for_range_bindings(
    locals: &mut HashMap<&String, cranelift_codegen::ir::Value>,
    carried: &[String],
    carried_outer: Vec<String>,
    exit_values: &[cranelift_codegen::ir::Value],
) {
    for name in carried_outer {
        if let Some((position, _)) =
            carried.iter().enumerate().find(|(_, candidate)| *candidate == &name)
            && let Some(binding_name) = locals.keys().find(|candidate| candidate.as_str() == name)
        {
            locals.insert(binding_name, exit_values[position]);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_for_array<'source>(
    function: &mut FunctionBuilder<'_>,
    binding: &'source ForBinding,
    collection: &crate::ast::Expr,
    block: &'source Block,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &mut HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    let array_id = match initializer_type(collection, types, functions, layouts)? {
        NativeType::Array(id) => id,
        _ => return Err(NativeEmitError("for source is not a fixed array".to_owned())),
    };
    let capacity = layouts
        .array(array_id)
        .map(|layout| layout.capacity)
        .ok_or_else(|| NativeEmitError("fixed array layout is unavailable".to_owned()))?;
    let _ = lower_expression(
        function,
        collection,
        locals,
        types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    let span = collection_span(collection);
    let start =
        crate::ast::Expr::Integer { value: "0".to_owned(), suffix: binding.ty.clone(), span };
    let end =
        crate::ast::Expr::Integer { value: capacity.to_string(), suffix: binding.ty.clone(), span };
    lower_for_range(
        function,
        binding,
        &start,
        &end,
        block,
        locals,
        types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )
}

fn collection_span(expression: &crate::ast::Expr) -> crate::lexer::SourceSpan {
    match expression {
        crate::ast::Expr::Identifier { span, .. }
        | crate::ast::Expr::Integer { span, .. }
        | crate::ast::Expr::BoolLiteral { span, .. }
        | crate::ast::Expr::BufferLiteral { span, .. }
        | crate::ast::Expr::FloatLiteral { span, .. }
        | crate::ast::Expr::StringLiteral { span, .. }
        | crate::ast::Expr::Grouping { span, .. }
        | crate::ast::Expr::Unary { span, .. }
        | crate::ast::Expr::Cast { span, .. }
        | crate::ast::Expr::Binary { span, .. }
        | crate::ast::Expr::Borrow { span, .. }
        | crate::ast::Expr::Try { span, .. }
        | crate::ast::Expr::Call { span, .. }
        | crate::ast::Expr::MethodCall { span, .. }
        | crate::ast::Expr::StructLit { span, .. }
        | crate::ast::Expr::FieldAccess { span, .. }
        | crate::ast::Expr::Index { span, .. }
        | crate::ast::Expr::Case { span, .. }
        | crate::ast::Expr::If { span, .. } => *span,
    }
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
