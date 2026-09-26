use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;

use super::super::enum_layout;
use crate::ast::{Expr, Role, Stmt};
use crate::semantic::LoopExitKind;

use super::super::cleanup::{emit_loop_cleanup, emit_return_cleanup};
use super::super::expressions::{initializer_type, lower_expression};
use super::super::layout::LayoutRegistry;
use super::super::literals::StringDataValues;
use super::super::native::{FunctionRef, NativeEmitError};
use super::super::structs::{emit_binding_drop, lower_field_assignment};
use super::super::types::NativeType;
use super::{Flow, LoopTargets, NativeCleanupSchedule};

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_statements<'source>(
    function: &mut FunctionBuilder<'_>,
    statements: &'source [Stmt],
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &mut HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    targets: Option<LoopTargets>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    for statement in statements {
        let flow = lower_statement(
            function,
            statement,
            locals,
            types,
            functions,
            targets.clone(),
            cleanup_schedule,
            string_data,
            layouts,
        )?;
        if !matches!(flow, Flow::Fallthrough) {
            return Ok(flow);
        }
    }
    Ok(Flow::Fallthrough)
}

#[allow(clippy::too_many_arguments)]
fn lower_statement<'source>(
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
        Stmt::OwnerDecl { role: Role::Erg | Role::Abs, name, ty, initializer, .. } =>
            lower_owner_declaration(function, name, ty.as_deref(), initializer, locals, types, functions, cleanup_schedule, string_data, layouts),
        Stmt::Assignment { name, value, .. } =>
            lower_assignment(function, name, value, locals, types, functions, cleanup_schedule, string_data, layouts),
        Stmt::FieldAssignment { object, field, value, .. } => lower_field_assignment(
            function, object, field, value, locals, types, functions, cleanup_schedule, string_data, layouts,
        ).map(|()| Flow::Fallthrough),
        Stmt::Return { value: Some(expression), span } => lower_return(
            function, expression, *span, locals, types, functions, cleanup_schedule, string_data,
            layouts,
        ),
        Stmt::Return { value: None, .. } => {
            Err(NativeEmitError("native function requires a return value".to_owned()))
        }
        Stmt::Expression { expression, .. } => {
            lower_expression(function, expression, locals, types, functions, cleanup_schedule, string_data, layouts)?;
            Ok(Flow::Fallthrough)
        }
        Stmt::Drop { name, .. } => lower_drop(function, name, locals, types, functions, layouts),
        Stmt::Block(block) => super::lowering_scopes::lower_scoped_block(
            function, block, locals, types, functions, targets, cleanup_schedule, string_data, layouts,
        ),
        Stmt::Loop(block) => super::lowering_loops::lower_loop(
            function, block, locals, types, functions, cleanup_schedule, string_data, layouts,
        ),
        Stmt::Break { span } => {
            emit_loop_cleanup(function, cleanup_schedule, *span, LoopExitKind::Break, locals, types, functions, layouts)?;
            super::lowering_loops::emit_loop_jump(function, targets, locals, false)
        }
        Stmt::Continue { span } => {
            emit_loop_cleanup(function, cleanup_schedule, *span, LoopExitKind::Continue, locals, types, functions, layouts)?;
            super::lowering_loops::emit_loop_jump(function, targets, locals, true)
        }
        _ => Err(NativeEmitError(
            "native integer slice supports only integer declarations, assignments, expressions, blocks, drops, and returns".to_owned(),
        )),
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_owner_declaration<'source>(
    function: &mut FunctionBuilder<'_>,
    name: &'source String,
    declared_type: Option<&str>,
    initializer: &Expr,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &mut HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    let value = lower_expression(
        function,
        initializer,
        locals,
        types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    locals.insert(name, value);
    let native_type = declared_type
        .and_then(|name| NativeType::from_name(name).or_else(|| layouts.type_for_name(name)))
        .unwrap_or_else(|| initializer_type(initializer, types, functions, layouts));
    types.insert(name, native_type);
    Ok(Flow::Fallthrough)
}

#[allow(clippy::too_many_arguments)]
fn lower_assignment<'source>(
    function: &mut FunctionBuilder<'_>,
    name: &'source String,
    expression: &Expr,
    locals: &mut HashMap<&'source String, cranelift_codegen::ir::Value>,
    types: &HashMap<&'source String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    let value = lower_expression(
        function,
        expression,
        locals,
        types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    locals.insert(name, value);
    Ok(Flow::Fallthrough)
}

#[allow(clippy::too_many_arguments)]
fn lower_return(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    span: crate::lexer::SourceSpan,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    if let Expr::Try { expression, .. } = expression {
        return lower_try_return(
            function,
            expression,
            span,
            locals,
            types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        );
    }
    let value = lower_expression(
        function,
        expression,
        locals,
        types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    emit_return_cleanup(function, cleanup_schedule, span, locals, types, functions, layouts)?;
    Ok(Flow::Return(value))
}

#[allow(clippy::too_many_arguments)]
fn lower_try_return(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    span: crate::lexer::SourceSpan,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    let source = lower_expression(
        function,
        expression,
        locals,
        types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    let NativeType::Enum(enum_id) = initializer_type(expression, types, functions, layouts) else {
        return Err(NativeEmitError("try operand is not a native Result value".to_owned()));
    };
    let layout = layouts
        .enum_layout(enum_id)
        .ok_or_else(|| NativeEmitError("try operand has no enum layout".to_owned()))?;
    let ok = layouts
        .enum_variant(enum_id, "Ok")
        .ok_or_else(|| NativeEmitError("Result enum has no Ok variant".to_owned()))?;
    let result_slot = function.create_sized_stack_slot(layouts.enum_stack_slot(layout));
    let result = function.ins().stack_addr(layouts.pointer_type, result_slot, 0);
    let discriminant = function.ins().load(
        types::I32,
        MemFlagsData::new(),
        source,
        layout.discriminant_offset as i32,
    );
    let is_ok = function.ins().icmp_imm_s(IntCC::Equal, discriminant, i64::from(ok.discriminant));
    let ok_block = function.create_block();
    let err_block = function.create_block();
    let merge = function.create_block();
    function.append_block_param(merge, layouts.pointer_type);
    function.ins().brif(is_ok, ok_block, &[], err_block, &[]);
    function.switch_to_block(ok_block);
    write_ok_result(function, source, result, layout, ok, layouts);
    let ok_argument = cranelift_codegen::ir::BlockArg::Value(result);
    function.ins().jump(merge, [&ok_argument]);
    function.seal_block(ok_block);
    function.switch_to_block(err_block);
    copy_enum_bytes(function, source, result, layout.size);
    let err_argument = cranelift_codegen::ir::BlockArg::Value(result);
    function.ins().jump(merge, [&err_argument]);
    function.seal_block(err_block);
    function.switch_to_block(merge);
    function.seal_block(merge);
    emit_return_cleanup(function, cleanup_schedule, span, locals, types, functions, layouts)?;
    Ok(Flow::Return(function.block_params(merge)[0]))
}

fn copy_enum_bytes(
    function: &mut FunctionBuilder<'_>,
    source: cranelift_codegen::ir::Value,
    destination: cranelift_codegen::ir::Value,
    size: u32,
) {
    copy_bytes(function, source, destination, 0, 0, size);
}

fn write_ok_result(
    function: &mut FunctionBuilder<'_>,
    source: cranelift_codegen::ir::Value,
    destination: cranelift_codegen::ir::Value,
    layout: &enum_layout::EnumLayout,
    ok: &enum_layout::EnumVariantLayout,
    layouts: &LayoutRegistry,
) {
    let discriminant = function.ins().iconst(types::I32, i64::from(ok.discriminant));
    function.ins().store(
        MemFlagsData::new(),
        discriminant,
        destination,
        layout.discriminant_offset as i32,
    );
    if let Some(field) = ok.fields.first()
        && let Some(size) = layouts.type_size(field.ty)
    {
        let offset = layout.payload_offset + field.offset;
        copy_bytes(function, source, destination, offset, offset, size);
    }
}

fn copy_bytes(
    function: &mut FunctionBuilder<'_>,
    source: cranelift_codegen::ir::Value,
    destination: cranelift_codegen::ir::Value,
    source_offset: u32,
    destination_offset: u32,
    size: u32,
) {
    for offset in 0..size {
        let byte = function.ins().load(
            types::I8,
            MemFlagsData::new(),
            source,
            (source_offset + offset) as i32,
        );
        function.ins().store(
            MemFlagsData::new(),
            byte,
            destination,
            (destination_offset + offset) as i32,
        );
    }
}

fn lower_drop(
    function: &mut FunctionBuilder<'_>,
    name: &str,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<Flow, NativeEmitError> {
    emit_binding_drop(function, name, locals, types, functions, layouts)?;
    Ok(Flow::Fallthrough)
}
