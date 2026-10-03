use std::collections::HashMap;

use cranelift_codegen::ir::InstBuilder;
use cranelift_frontend::FunctionBuilder;

use crate::ast::{Block, Expr, IfBranch};

use super::super::literals::StringDataValues;
use super::super::lowering::LoopTargets;
use super::super::lowering::{Flow, lower_statements};
use super::super::model::NativeCleanupSchedule;
use super::super::native::FunctionRef;
use super::super::native::NativeEmitError;
use super::super::types::NativeType;
use super::CallLoweringContext;
use super::{coerce_to_ir_type, initializer_type, lower_expression_with_context};

#[allow(clippy::too_many_arguments)]
pub(crate) fn lower_statement_if<'source>(
    function: &mut FunctionBuilder<'_>,
    condition: &Expr,
    then_branch: &'source Block,
    else_branch: Option<&'source IfBranch>,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    local_types: &mut HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &crate::codegen::layout::LayoutRegistry,
    targets: Option<LoopTargets>,
) -> Result<super::super::lowering::Flow, NativeEmitError> {
    let condition = super::lower_expression_with_targets(
        function,
        condition,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
        targets.clone(),
    )?;
    let bindings = locals.keys().copied().collect::<Vec<_>>();
    let (then_block, else_block, merge_block) =
        create_statement_if_blocks(function, &bindings, local_types, layouts)?;
    function.ins().brif(condition, then_block, &[], else_block, &[]);
    lower_statement_if_branches(
        function,
        then_block,
        then_branch,
        else_block,
        else_branch,
        locals,
        local_types,
        &bindings,
        functions,
        targets,
        cleanup_schedule,
        string_data,
        layouts,
        merge_block,
    )?;
    function.switch_to_block(merge_block);
    update_statement_if_bindings(function, merge_block, &bindings, locals);
    function.seal_block(then_block);
    function.seal_block(else_block);
    function.seal_block(merge_block);
    Ok(super::super::lowering::Flow::Fallthrough)
}

#[allow(clippy::too_many_arguments)]
fn lower_statement_if_branches<'source>(
    function: &mut FunctionBuilder<'_>,
    then_block: cranelift_codegen::ir::Block,
    then_branch: &'source Block,
    else_block: cranelift_codegen::ir::Block,
    else_branch: Option<&'source IfBranch>,
    locals: &HashMap<&'source String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&'source String, NativeType>,
    bindings: &[&'source String],
    functions: &HashMap<String, FunctionRef>,
    targets: Option<LoopTargets>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &crate::codegen::layout::LayoutRegistry,
    merge_block: cranelift_codegen::ir::Block,
) -> Result<(), NativeEmitError> {
    lower_statement_if_branch(
        function,
        then_block,
        then_branch,
        locals,
        local_types,
        bindings,
        functions,
        targets.clone(),
        cleanup_schedule,
        string_data,
        layouts,
        merge_block,
    )?;
    lower_statement_if_else_branch(
        function,
        else_block,
        else_branch,
        locals,
        local_types,
        bindings,
        functions,
        targets,
        cleanup_schedule,
        string_data,
        layouts,
        merge_block,
    )
}

fn update_statement_if_bindings<'source>(
    function: &FunctionBuilder<'_>,
    merge_block: cranelift_codegen::ir::Block,
    bindings: &[&'source String],
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
) {
    for (binding, value) in bindings.iter().zip(function.block_params(merge_block)) {
        locals.insert(*binding, *value);
    }
}

fn create_statement_if_blocks<'source>(
    function: &mut FunctionBuilder<'_>,
    bindings: &[&'source String],
    local_types: &HashMap<&'source String, NativeType>,
    layouts: &crate::codegen::layout::LayoutRegistry,
) -> Result<
    (cranelift_codegen::ir::Block, cranelift_codegen::ir::Block, cranelift_codegen::ir::Block),
    NativeEmitError,
> {
    let then_block = function.create_block();
    let else_block = function.create_block();
    let merge_block = function.create_block();
    for binding in bindings {
        let ty = local_types
            .get(binding)
            .copied()
            .ok_or_else(|| NativeEmitError(format!("missing native type for `{binding}")))?;
        function.append_block_param(merge_block, ty.ir_type(layouts.pointer_type)?);
    }
    Ok((then_block, else_block, merge_block))
}

#[allow(clippy::too_many_arguments)]
fn lower_statement_if_else_branch<'source>(
    function: &mut FunctionBuilder<'_>,
    else_block: cranelift_codegen::ir::Block,
    else_branch: Option<&'source IfBranch>,
    locals: &HashMap<&'source String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&'source String, NativeType>,
    bindings: &[&'source String],
    functions: &HashMap<String, FunctionRef>,
    targets: Option<LoopTargets>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &crate::codegen::layout::LayoutRegistry,
    merge_block: cranelift_codegen::ir::Block,
) -> Result<(), NativeEmitError> {
    match else_branch {
        Some(IfBranch::Block(block)) => lower_statement_if_branch(
            function,
            else_block,
            block,
            locals,
            local_types,
            bindings,
            functions,
            targets,
            cleanup_schedule,
            string_data,
            layouts,
            merge_block,
        ),
        None => {
            function.switch_to_block(else_block);
            jump_with_bindings(function, merge_block, bindings, locals);
            Ok(())
        }
        Some(IfBranch::ElseIf(_)) => {
            Err(NativeEmitError("statement if lowering requires a block else branch".to_owned()))
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_statement_if_branch<'source>(
    function: &mut FunctionBuilder<'_>,
    branch_block: cranelift_codegen::ir::Block,
    block: &'source Block,
    locals: &HashMap<&'source String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&'source String, NativeType>,
    bindings: &[&'source String],
    functions: &HashMap<String, FunctionRef>,
    targets: Option<LoopTargets>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &crate::codegen::layout::LayoutRegistry,
    merge_block: cranelift_codegen::ir::Block,
) -> Result<(), NativeEmitError> {
    function.switch_to_block(branch_block);
    let mut branch_locals = locals.clone();
    let mut branch_types = local_types.clone();
    let flow = lower_statements(
        function,
        &block.statements,
        &mut branch_locals,
        &mut branch_types,
        functions,
        targets,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    if !matches!(flow, Flow::Fallthrough) {
        return Err(NativeEmitError(
            "non-fallthrough control flow requires value-preserving statement lowering".to_owned(),
        ));
    }
    let values = bindings
        .iter()
        .map(|binding| {
            branch_locals.get(binding).copied().ok_or_else(|| {
                NativeEmitError(format!("binding `{binding}` disappeared in if branch"))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let args = values
        .iter()
        .map(|value| cranelift_codegen::ir::BlockArg::Value(*value))
        .collect::<Vec<_>>();
    function.ins().jump(merge_block, &args);
    Ok(())
}

fn jump_with_bindings(
    function: &mut FunctionBuilder<'_>,
    merge_block: cranelift_codegen::ir::Block,
    bindings: &[&String],
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
) {
    let args = bindings
        .iter()
        .filter_map(|binding| locals.get(binding).copied())
        .map(cranelift_codegen::ir::BlockArg::Value);
    function.ins().jump(merge_block, &args.collect::<Vec<_>>());
}

pub(super) fn lower_if(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    context: &CallLoweringContext<'_, '_>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let Expr::If { condition, then_branch, else_branch, .. } = expression else { unreachable!() };
    let condition = lower_expression_with_context(function, condition, context)?;
    let then_block = function.create_block();
    let else_block = function.create_block();
    let merge_block = function.create_block();
    let result_type =
        initializer_type(expression, context.local_types, context.functions, context.layouts)?;
    function.append_block_param(merge_block, context.layouts.ir_type(result_type)?);
    function.ins().brif(condition, then_block, &[], else_block, &[]);

    lower_branch(function, then_block, then_branch, context, merge_block, result_type)?;
    function.switch_to_block(else_block);
    match else_branch {
        Some(IfBranch::Block(block)) => {
            lower_branch(function, else_block, block, context, merge_block, result_type)?;
        }
        Some(IfBranch::ElseIf(nested)) => {
            let nested_value = lower_if(function, nested, context)?;
            if !function.is_unreachable() {
                let value = coerce_to_ir_type(
                    function,
                    nested_value,
                    context.layouts.ir_type(result_type)?,
                );
                let argument = cranelift_codegen::ir::BlockArg::Value(value);
                function.ins().jump(merge_block, [&argument]);
            }
        }
        None => {
            emit_merge_value(function, merge_block, result_type, context)?;
        }
    }

    function.switch_to_block(merge_block);
    function.seal_block(then_block);
    function.seal_block(else_block);
    function.seal_block(merge_block);
    Ok(function.block_params(merge_block)[0])
}

fn lower_branch(
    function: &mut FunctionBuilder<'_>,
    branch_block: cranelift_codegen::ir::Block,
    block: &crate::ast::Block,
    context: &CallLoweringContext<'_, '_>,
    merge_block: cranelift_codegen::ir::Block,
    result_type: NativeType,
) -> Result<(), NativeEmitError> {
    function.switch_to_block(branch_block);
    let mut locals = context.locals.clone();
    let mut local_types = context.local_types.clone();
    let (prefix, tail) = split_tail_expression(block);
    let flow = lower_statements(
        function,
        prefix,
        &mut locals,
        &mut local_types,
        context.functions,
        context.loop_targets.clone(),
        context.cleanup_schedule,
        context.string_data,
        context.layouts,
    )?;
    if matches!(flow, Flow::Fallthrough) {
        let branch_context = CallLoweringContext::new(
            &locals,
            &local_types,
            context.functions,
            context.cleanup_schedule,
            context.string_data,
            context.layouts,
        )
        .with_loop_targets(context.loop_targets.clone());
        let value = tail
            .map(|expression| lower_expression_with_context(function, expression, &branch_context))
            .transpose()?
            .map(|value| {
                coerce_to_ir_type(function, value, context.layouts.ir_type(result_type).unwrap())
            });
        let value = value.unwrap_or_else(|| {
            function.ins().iconst(context.layouts.ir_type(result_type).unwrap(), 0)
        });
        let argument = cranelift_codegen::ir::BlockArg::Value(value);
        function.ins().jump(merge_block, [&argument]);
    } else {
        emit_branch_flow(function, flow, result_type, context.layouts)?;
    }
    Ok(())
}

fn emit_branch_flow(
    function: &mut FunctionBuilder<'_>,
    flow: Flow,
    result_type: NativeType,
    layouts: &crate::codegen::layout::LayoutRegistry,
) -> Result<(), NativeEmitError> {
    match flow {
        Flow::Return(value, value_type) => {
            let value_type = value_type.unwrap_or(result_type);
            let return_slot = if layouts.uses_return_slot(value_type) {
                let entry = function.func.layout.entry_block().ok_or_else(|| {
                    NativeEmitError("native function has no entry block".to_owned())
                })?;
                function.block_params(entry).first().copied()
            } else {
                None
            };
            crate::codegen::function_definition::emit_value_return(
                function,
                Some(value_type),
                return_slot,
                value,
                layouts,
            )
        }
        Flow::VoidReturn => {
            function.ins().return_(&[]);
            Ok(())
        }
        Flow::Break | Flow::Continue => Ok(()),
        Flow::Fallthrough => Ok(()),
    }
}

fn split_tail_expression(block: &crate::ast::Block) -> (&[crate::ast::Stmt], Option<&Expr>) {
    let Some(crate::ast::Stmt::Expression { expression, span }) = block.statements.last() else {
        return (&block.statements, None);
    };
    if span.end != expression_end(expression) {
        return (&block.statements, None);
    }
    let split = block.statements.len() - 1;
    (&block.statements[..split], Some(expression))
}

fn expression_end(expression: &Expr) -> usize {
    match expression {
        Expr::Identifier { span, .. }
        | Expr::Integer { span, .. }
        | Expr::BoolLiteral { span, .. }
        | Expr::BufferLiteral { span, .. }
        | Expr::FloatLiteral { span, .. }
        | Expr::StringLiteral { span, .. }
        | Expr::Grouping { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Cast { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Borrow { span, .. }
        | Expr::Try { span, .. }
        | Expr::Call { span, .. }
        | Expr::MethodCall { span, .. }
        | Expr::StructLit { span, .. }
        | Expr::FieldAccess { span, .. }
        | Expr::Index { span, .. }
        | Expr::Case { span, .. }
        | Expr::If { span, .. } => span.end,
    }
}

fn emit_merge_value(
    function: &mut FunctionBuilder<'_>,
    merge_block: cranelift_codegen::ir::Block,
    result_type: NativeType,
    context: &CallLoweringContext<'_, '_>,
) -> Result<(), NativeEmitError> {
    let value = function.ins().iconst(context.layouts.ir_type(result_type)?, 0);
    let argument = cranelift_codegen::ir::BlockArg::Value(value);
    function.ins().jump(merge_block, [&argument]);
    Ok(())
}
