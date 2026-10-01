use cranelift_codegen::ir::{InstBuilder, types};
use cranelift_frontend::FunctionBuilder;

use crate::ast::{Expr, IfBranch};

use super::super::lowering::{Flow, lower_statements};
use super::super::native::NativeEmitError;
use super::CallLoweringContext;
use super::lower_expression_with_context;

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
    function.ins().brif(condition, then_block, &[], else_block, &[]);

    lower_branch(function, then_block, then_branch, context, merge_block)?;
    function.switch_to_block(else_block);
    match else_branch {
        Some(IfBranch::Block(block)) => {
            lower_branch(function, else_block, block, context, merge_block)?;
        }
        Some(IfBranch::ElseIf(nested)) => {
            let _ = lower_if(function, nested, context)?;
            if !function.is_unreachable() {
                function.ins().jump(merge_block, &[]);
            }
        }
        None => {
            function.ins().jump(merge_block, &[]);
        }
    }

    function.switch_to_block(merge_block);
    function.seal_block(then_block);
    function.seal_block(else_block);
    function.seal_block(merge_block);
    Ok(function.ins().iconst(types::I64, 0))
}

fn lower_branch(
    function: &mut FunctionBuilder<'_>,
    branch_block: cranelift_codegen::ir::Block,
    block: &crate::ast::Block,
    context: &CallLoweringContext<'_, '_>,
    merge_block: cranelift_codegen::ir::Block,
) -> Result<(), NativeEmitError> {
    function.switch_to_block(branch_block);
    let mut locals = context.locals.clone();
    let mut local_types = context.local_types.clone();
    let flow = lower_statements(
        function,
        &block.statements,
        &mut locals,
        &mut local_types,
        context.functions,
        context.loop_targets.clone(),
        context.cleanup_schedule,
        context.string_data,
        context.layouts,
    )?;
    if matches!(flow, Flow::Fallthrough) {
        function.ins().jump(merge_block, &[]);
    }
    Ok(())
}
