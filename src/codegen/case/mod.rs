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
    locals: &mut HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &super::model::NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
    loop_targets: Option<super::lowering::LoopTargets>,
    value_producing: bool,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let context = case_context(
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
        loop_targets,
    );
    let subject_value = lower_case_subject(function, subject, &context)?;
    let merge = function.create_block();
    let bindings = locals
        .iter()
        .filter_map(|(binding, value)| {
            let declared = local_types.get(binding)?.ir_type(layouts.pointer_type).ok()?;
            (function.func.dfg.value_type(*value) == declared).then_some(*binding)
        })
        .collect::<Vec<_>>();
    for binding in &bindings {
        let ty = local_types
            .get(binding)
            .copied()
            .ok_or_else(|| NativeEmitError(format!("missing native type for `{binding}`")))?;
        function.append_block_param(merge, layouts.ir_type(ty)?);
    }
    let subject_type = initializer_type(subject, local_types, functions, layouts)?;
    let result_type =
        branch_type(branches, subject_type, local_types, functions, layouts, value_producing)?;
    function.append_block_param(merge, layouts.ir_type(result_type)?);
    branches::emit_case_branches(
        function,
        branches,
        subject_value,
        subject_type,
        result_type,
        merge,
        &bindings,
        &context,
        value_producing,
    )?;
    finish_case_merge(function, merge, locals, &bindings);
    Ok(*function.block_params(merge).last().expect("case merge result"))
}

fn finish_case_merge<'a>(
    function: &mut FunctionBuilder<'_>,
    merge: cranelift_codegen::ir::Block,
    locals: &mut HashMap<&'a String, cranelift_codegen::ir::Value>,
    bindings: &[&'a String],
) {
    function.switch_to_block(merge);
    for (binding, value) in bindings.iter().zip(function.block_params(merge)) {
        locals.insert(*binding, *value);
    }
    function.seal_block(merge);
}

fn case_context<'maps, 'keys>(
    locals: &'maps HashMap<&'keys String, cranelift_codegen::ir::Value>,
    local_types: &'maps HashMap<&'keys String, NativeType>,
    functions: &'maps HashMap<String, FunctionRef>,
    cleanup_schedule: &'maps super::model::NativeCleanupSchedule,
    string_data: &'maps StringDataValues,
    layouts: &'maps LayoutRegistry,
    loop_targets: Option<super::lowering::LoopTargets>,
) -> CaseLoweringContext<'maps, 'keys> {
    CaseLoweringContext::new(
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
        loop_targets,
    )
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
