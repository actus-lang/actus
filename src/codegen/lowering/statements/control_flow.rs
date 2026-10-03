use std::collections::HashMap;

use cranelift_frontend::FunctionBuilder;

use crate::ast::{Block, Expr, IfBranch, Stmt};

use super::super::super::layout::LayoutRegistry;
use super::super::super::literals::StringDataValues;
use super::super::super::native::{FunctionRef, NativeEmitError};
use super::super::super::types::NativeType;
use super::{Flow, LoopTargets, NativeCleanupSchedule};

pub(super) fn statement_if_can_merge(then_branch: &Block, else_branch: Option<&IfBranch>) -> bool {
    !block_has_control_flow(then_branch)
        && else_branch.is_none_or(|branch| match branch {
            IfBranch::Block(block) => !block_has_control_flow(block),
            IfBranch::ElseIf(_) => false,
        })
}

fn block_has_control_flow(block: &Block) -> bool {
    block.statements.iter().any(|statement| match statement {
        Stmt::Return { .. } | Stmt::Break { .. } | Stmt::Continue { .. } => true,
        Stmt::If { then_branch, else_branch, .. } => {
            block_has_control_flow(then_branch)
                || else_branch.as_ref().is_some_and(|branch| match branch {
                    IfBranch::Block(block) => block_has_control_flow(block),
                    IfBranch::ElseIf(_) => true,
                })
        }
        Stmt::Block(block) | Stmt::Loop(block) => block_has_control_flow(block),
        _ => false,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_if_statement<'source>(
    function: &mut FunctionBuilder<'_>,
    condition: &Expr,
    then_branch: &'source Block,
    else_branch: &'source Option<IfBranch>,
    span: crate::lexer::SourceSpan,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &mut HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    targets: Option<LoopTargets>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    if statement_if_can_merge(then_branch, else_branch.as_ref()) {
        return super::super::super::expressions::lower_statement_if(
            function,
            condition,
            then_branch,
            else_branch.as_ref(),
            locals,
            types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
            targets,
        );
    }
    let expression = Expr::If {
        condition: Box::new(condition.clone()),
        then_branch: then_branch.clone(),
        else_branch: else_branch.clone(),
        span,
    };
    super::lower_expression_statement(
        function,
        &expression,
        span,
        locals,
        types,
        functions,
        targets,
        cleanup_schedule,
        string_data,
        layouts,
    )
}
