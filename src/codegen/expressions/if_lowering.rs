use cranelift_codegen::ir::InstBuilder;
use cranelift_frontend::FunctionBuilder;

use crate::ast::{Expr, IfBranch};

use super::super::lowering::{Flow, lower_statements};
use super::super::native::NativeEmitError;
use super::super::types::NativeType;
use super::CallLoweringContext;
use super::{coerce_to_ir_type, initializer_type, lower_expression_with_context};

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
    }
    Ok(())
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
