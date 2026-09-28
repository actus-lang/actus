use std::collections::HashMap;

use cranelift_frontend::FunctionBuilder;

use crate::ast::Expr;

use super::expressions::{initializer_type, lower_expression};
use super::layout::LayoutRegistry;
use super::literals::StringDataValues;
use super::native::{FunctionRef, NativeEmitError};
use super::types::NativeType;

mod branches;
mod context;
mod matching;
mod payload;
mod types;

use self::context::CaseLoweringContext;
use self::types::branch_type;
pub(super) use types::infer_case_type;

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_case(
    function: &mut FunctionBuilder<'_>,
    subject: &Expr,
    branches: &[crate::ast::CaseBranch],
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &super::model::NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let context =
        case_context(locals, local_types, functions, cleanup_schedule, string_data, layouts);
    let subject_value = lower_case_subject(function, subject, &context)?;
    let merge = function.create_block();
    let subject_type = initializer_type(subject, local_types, functions, layouts)?;
    let result_type = branch_type(branches, subject_type, local_types, functions, layouts)?;
    function.append_block_param(merge, layouts.ir_type(result_type)?);
    branches::emit_case_branches(
        function,
        branches,
        subject_value,
        subject_type,
        result_type,
        merge,
        &context,
    )?;
    finish_case_merge(function, merge);
    Ok(function.block_params(merge)[0])
}

fn finish_case_merge(function: &mut FunctionBuilder<'_>, merge: cranelift_codegen::ir::Block) {
    function.switch_to_block(merge);
    function.seal_block(merge);
}

fn case_context<'maps, 'keys>(
    locals: &'maps HashMap<&'keys String, cranelift_codegen::ir::Value>,
    local_types: &'maps HashMap<&'keys String, NativeType>,
    functions: &'maps HashMap<String, FunctionRef>,
    cleanup_schedule: &'maps super::model::NativeCleanupSchedule,
    string_data: &'maps StringDataValues,
    layouts: &'maps LayoutRegistry,
) -> CaseLoweringContext<'maps, 'keys> {
    CaseLoweringContext::new(locals, local_types, functions, cleanup_schedule, string_data, layouts)
}

fn lower_case_subject(
    function: &mut FunctionBuilder<'_>,
    subject: &Expr,
    context: &CaseLoweringContext<'_, '_>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    lower_expression(
        function,
        subject,
        context.locals,
        context.local_types,
        context.functions,
        context.cleanup_schedule,
        context.string_data,
        context.layouts,
    )
}
