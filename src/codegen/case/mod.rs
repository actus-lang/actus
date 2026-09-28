use std::collections::HashMap;

use cranelift_frontend::FunctionBuilder;

use crate::ast::Expr;

use super::expressions::{initializer_type, lower_expression};
use super::layout::LayoutRegistry;
use super::literals::StringDataValues;
use super::native::{FunctionRef, NativeEmitError};
use super::types::NativeType;

mod branches;
mod matching;
mod payload;
mod types;

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
    let subject_value = lower_expression(
        function,
        subject,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
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
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    function.switch_to_block(merge);
    function.seal_block(merge);
    Ok(function.block_params(merge)[0])
}
