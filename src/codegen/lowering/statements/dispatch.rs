use std::collections::HashMap;

use cranelift_frontend::FunctionBuilder;

use crate::ast::{Role, Stmt};

use super::super::super::layout::LayoutRegistry;
use super::super::super::literals::StringDataValues;
use super::super::super::native::{FunctionRef, NativeEmitError};
use super::super::super::types::NativeType;
use super::super::{Flow, LoopTargets, NativeCleanupSchedule};

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_statement<'source>(
    function: &mut FunctionBuilder<'_>,
    statement: &'source Stmt,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &mut HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    targets: Option<LoopTargets>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    if let Some(flow) = lower_value_statement(
        function,
        statement,
        locals,
        types,
        functions,
        targets.clone(),
        cleanup_schedule,
        string_data,
        layouts,
    )? {
        return Ok(flow);
    }
    lower_control_statement(
        function,
        statement,
        locals,
        types,
        functions,
        targets,
        cleanup_schedule,
        string_data,
        layouts,
    )
}

#[allow(clippy::too_many_arguments)]
fn lower_value_statement<'source>(
    function: &mut FunctionBuilder<'_>,
    statement: &'source Stmt,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &mut HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    targets: Option<LoopTargets>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Option<Flow>, NativeEmitError> {
    if !matches!(
        statement,
        Stmt::OwnerDecl { role: Role::Erg | Role::Abs | Role::Ins, .. }
            | Stmt::Assignment { .. }
            | Stmt::CompoundAssignment { .. }
    ) {
        return lower_other_value_statement(
            function,
            statement,
            locals,
            types,
            functions,
            targets,
            cleanup_schedule,
            string_data,
            layouts,
        );
    }
    Ok(Some(lower_owner_or_assignment(
        function,
        statement,
        locals,
        types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?))
}

#[allow(clippy::too_many_arguments)]
fn lower_owner_or_assignment<'source>(
    function: &mut FunctionBuilder<'_>,
    statement: &'source Stmt,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &mut HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    match statement {
        Stmt::OwnerDecl {
            role: Role::Erg | Role::Abs | Role::Ins, name, ty, initializer, ..
        } => super::owners::lower_owner_declaration(
            function,
            name,
            ty.as_deref(),
            initializer,
            locals,
            types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
        Stmt::Assignment { name, value, .. } => super::lower_assignment(
            function,
            name,
            value,
            locals,
            types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
        Stmt::CompoundAssignment { target, operator, value, .. } => {
            super::lower_compound_assignment(
                function,
                target,
                *operator,
                value,
                locals,
                types,
                functions,
                cleanup_schedule,
                string_data,
                layouts,
            )
        }
        _ => unreachable!(),
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_other_value_statement<'source>(
    function: &mut FunctionBuilder<'_>,
    statement: &'source Stmt,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &mut HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    targets: Option<LoopTargets>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Option<Flow>, NativeEmitError> {
    match statement {
        Stmt::FieldAssignment { .. } | Stmt::IndexAssignment { .. } | Stmt::Return { .. } => {
            Ok(Some(lower_field_or_return(
                function,
                statement,
                locals,
                types,
                functions,
                cleanup_schedule,
                string_data,
                layouts,
            )?))
        }
        Stmt::Expression { .. } | Stmt::Drop { .. } => Ok(Some(lower_expression_or_drop(
            function,
            statement,
            locals,
            types,
            functions,
            targets,
            cleanup_schedule,
            string_data,
            layouts,
        )?)),
        _ => Ok(None),
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_field_or_return<'source>(
    function: &mut FunctionBuilder<'_>,
    statement: &'source Stmt,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &mut HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    match statement {
        Stmt::FieldAssignment { .. } => lower_field_assignment_statement(
            function,
            statement,
            locals,
            types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
        Stmt::IndexAssignment { target, index, value, .. } => {
            super::super::super::arrays::lower_array_assignment(
                function,
                target,
                index,
                value,
                locals,
                types,
                functions,
                cleanup_schedule,
                string_data,
                layouts,
            )
            .map(|()| Flow::Fallthrough)
        }
        Stmt::Return { value, span } => super::lower_return_statement(
            function,
            value.as_ref(),
            *span,
            locals,
            types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
        _ => unreachable!(),
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_field_assignment_statement<'source>(
    function: &mut FunctionBuilder<'_>,
    statement: &'source Stmt,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &mut HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    let Stmt::FieldAssignment { object, field, value, .. } = statement else { unreachable!() };
    super::super::super::structs::lower_field_assignment(
        function,
        object,
        field,
        value,
        locals,
        types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )
    .map(|()| Flow::Fallthrough)
}

#[allow(clippy::too_many_arguments)]
fn lower_expression_or_drop<'source>(
    function: &mut FunctionBuilder<'_>,
    statement: &'source Stmt,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    targets: Option<LoopTargets>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    match statement {
        Stmt::Expression { expression, span } => super::lower_expression_statement(
            function,
            expression,
            *span,
            locals,
            types,
            functions,
            targets,
            cleanup_schedule,
            string_data,
            layouts,
        ),
        Stmt::Drop { name, .. } => {
            super::lower_drop(function, name, locals, types, functions, layouts)
        }
        _ => unreachable!(),
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_control_statement<'source>(
    function: &mut FunctionBuilder<'_>,
    statement: &'source Stmt,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &mut HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    targets: Option<LoopTargets>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    match statement {
        Stmt::Block(block) => super::super::scopes::lower_scoped_block(
            function, block, locals, types, functions, targets, cleanup_schedule, string_data, layouts,
        ),
        Stmt::Loop(block) => super::super::loops::lower_loop(
            function, block, locals, types, functions, cleanup_schedule, string_data, layouts,
        ),
        Stmt::Break { span } | Stmt::Continue { span } => lower_loop_exit(
            function,
            *span,
            targets,
            locals,
            types,
            functions,
            cleanup_schedule,
            layouts,
            matches!(statement, Stmt::Continue { .. }),
        ),
        _ => Err(NativeEmitError(
            "native integer slice supports only integer declarations, assignments, expressions, blocks, drops, and returns".to_owned(),
        )),
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_loop_exit<'source>(
    function: &mut FunctionBuilder<'_>,
    span: crate::lexer::SourceSpan,
    targets: Option<LoopTargets>,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    layouts: &LayoutRegistry,
    is_continue: bool,
) -> Result<Flow, NativeEmitError> {
    super::lower_loop_control(
        function,
        span,
        targets,
        locals,
        types,
        functions,
        cleanup_schedule,
        layouts,
        is_continue,
    )
}
