use cranelift_codegen::ir::{BlockArg, InstBuilder};
use cranelift_frontend::FunctionBuilder;

use crate::ast::{BinaryOp, Expr};

use super::super::calls::CallLoweringContext;
use super::super::native::NativeEmitError;
use super::operations::lower_expression_with_context;

pub(super) fn lower_short_circuit(
    function: &mut FunctionBuilder<'_>,
    left_expression: &Expr,
    operator: &BinaryOp,
    right_expression: &Expr,
    context: &CallLoweringContext<'_, '_>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let left = lower_expression_with_context(function, left_expression, context)?;
    let value_type = function.func.dfg.value_type(left);
    let right_block = function.create_block();
    let short_block = function.create_block();
    let merge = function.create_block();
    function.append_block_param(merge, value_type);

    let short_value = match operator {
        BinaryOp::LogicalAnd => 0,
        BinaryOp::LogicalOr => 1,
        _ => return Err(NativeEmitError("non-logical short-circuit operator".to_owned())),
    };
    let short_value = function.ins().iconst(value_type, short_value);
    let (true_target, false_target) = match operator {
        BinaryOp::LogicalAnd => (right_block, short_block),
        BinaryOp::LogicalOr => (short_block, right_block),
        _ => unreachable!(),
    };
    function.ins().brif(left, true_target, &[], false_target, &[]);

    function.switch_to_block(short_block);
    let short_argument = BlockArg::Value(short_value);
    function.ins().jump(merge, [&short_argument]);
    function.seal_block(short_block);

    function.switch_to_block(right_block);
    let right = lower_expression_with_context(function, right_expression, context)?;
    let right_argument = BlockArg::Value(right);
    function.ins().jump(merge, [&right_argument]);
    function.seal_block(right_block);

    function.switch_to_block(merge);
    function.seal_block(merge);
    Ok(function.block_params(merge)[0])
}
