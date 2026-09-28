use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;

use super::super::super::cleanup::emit_return_cleanup;
use super::super::super::enum_layout;
use super::super::super::expressions::{coerce_to_ir_type, lower_float_as};
use super::super::super::expressions::{initializer_type, lower_expression};
use super::super::super::layout::LayoutRegistry;
use super::super::super::literals::StringDataValues;
use super::super::super::native::{FunctionRef, NativeEmitError};
use super::super::super::types::NativeType;
use super::super::{Flow, NativeCleanupSchedule};
use crate::ast::Expr;

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_return(
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
    let return_ir_type = function.func.signature.returns.first().map(|ret| ret.value_type);
    let value =
        if let (Some(target), Expr::FloatLiteral { value, .. }) = (return_ir_type, expression) {
            lower_float_as(function, value, target)?
        } else {
            lower_expression(
                function,
                expression,
                locals,
                types,
                functions,
                cleanup_schedule,
                string_data,
                layouts,
            )?
        };
    let value = return_ir_type.map_or(value, |target| coerce_to_ir_type(function, value, target));
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
    let NativeType::Enum(enum_id) = initializer_type(expression, types, functions, layouts)? else {
        return Err(NativeEmitError("try operand is not a native Result value".to_owned()));
    };
    let layout = layouts
        .enum_layout(enum_id)
        .ok_or_else(|| NativeEmitError("try operand has no enum layout".to_owned()))?;
    let ok = layouts
        .enum_variant(enum_id, "Ok")
        .ok_or_else(|| NativeEmitError("Result enum has no Ok variant".to_owned()))?;
    let result = allocate_enum(function, layout.size, functions, layouts)?;
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

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_try_statement(
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
    let source_expression = match expression {
        Expr::Try { expression, .. } => expression.as_ref(),
        _ => return Err(NativeEmitError("expected try expression".to_owned())),
    };
    let source = lower_expression(
        function,
        source_expression,
        locals,
        types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    let NativeType::Enum(enum_id) = initializer_type(source_expression, types, functions, layouts)?
    else {
        return Err(NativeEmitError("try operand is not a native Result value".to_owned()));
    };
    let layout = layouts
        .enum_layout(enum_id)
        .ok_or_else(|| NativeEmitError("try operand has no enum layout".to_owned()))?;
    let ok = layouts
        .enum_variant(enum_id, "Ok")
        .ok_or_else(|| NativeEmitError("Result enum has no Ok variant".to_owned()))?;
    let discriminant = function.ins().load(
        types::I32,
        MemFlagsData::new(),
        source,
        layout.discriminant_offset as i32,
    );
    let is_ok = function.ins().icmp_imm_s(IntCC::Equal, discriminant, i64::from(ok.discriminant));
    let ok_block = function.create_block();
    let err_block = function.create_block();
    let continuation = function.create_block();
    function.ins().brif(is_ok, ok_block, &[], err_block, &[]);
    function.switch_to_block(err_block);
    emit_return_cleanup(function, cleanup_schedule, span, locals, types, functions, layouts)?;
    function.ins().return_(&[source]);
    function.seal_block(err_block);
    function.switch_to_block(ok_block);
    function.ins().jump(continuation, &[]);
    function.seal_block(ok_block);
    function.switch_to_block(continuation);
    function.seal_block(continuation);
    Ok(Flow::Fallthrough)
}

fn copy_enum_bytes(
    function: &mut FunctionBuilder<'_>,
    source: cranelift_codegen::ir::Value,
    destination: cranelift_codegen::ir::Value,
    size: u32,
) {
    copy_bytes(function, source, destination, 0, 0, size);
}

fn allocate_enum(
    function: &mut FunctionBuilder<'_>,
    size: u32,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let allocator = functions
        .get("__actus_enum_allocate")
        .ok_or_else(|| NativeEmitError("native enum allocator is unavailable".to_owned()))?;
    let size = function.ins().iconst(layouts.pointer_type, i64::from(size));
    let call = function.ins().call(allocator.reference, &[size]);
    function
        .inst_results(call)
        .first()
        .copied()
        .ok_or_else(|| NativeEmitError("native enum allocator returned no value".to_owned()))
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
