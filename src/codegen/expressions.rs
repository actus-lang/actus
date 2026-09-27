use std::collections::HashMap;

use cranelift_codegen::ir::{InstBuilder, MemFlagsData, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;

use crate::ast::Expr;

use super::case::lower_case;
use super::cleanup::emit_return_cleanup;
use super::enums::enum_expression_type;
use super::expression_construct::lower_construct;
use super::expression_literals::{lower_float, lower_identifier, lower_integer, lower_string};
use super::expression_operations::lower_operation;
use super::layout::LayoutRegistry;
use super::literals::StringDataValues;
use super::model::NativeCleanupSchedule;
use super::native::{FunctionRef, NativeEmitError};
use super::structs::{expression_native_type, field_type};
use super::types::NativeType;

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_expression(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    match expression {
        Expr::Integer { value, .. } => lower_integer(function, value),
        Expr::BufferLiteral { length, .. } => lower_buffer_literal(
            function,
            length,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
        Expr::FloatLiteral { value, .. } => lower_float(function, value),
        Expr::StringLiteral { value, .. } => lower_string(function, value, string_data),
        Expr::Identifier { name, .. } => lower_identifier(name, locals),
        Expr::Grouping { expression, .. } | Expr::Borrow { expression, .. } => lower_expression(
            function,
            expression,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
        _ => lower_complex_expression(
            function,
            expression,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_buffer_literal(
    function: &mut FunctionBuilder<'_>,
    length: &Expr,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let length = lower_expression(
        function,
        length,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    let length = if function.func.dfg.value_type(length) == layouts.pointer_type {
        length
    } else {
        function.ins().uextend(layouts.pointer_type, length)
    };
    let allocator = functions
        .get("__actus_buffer_allocate")
        .ok_or_else(|| NativeEmitError("native buffer allocator is unavailable".to_owned()))?;
    let call = function.ins().call(allocator.reference, &[length]);
    function
        .inst_results(call)
        .first()
        .copied()
        .ok_or_else(|| NativeEmitError("native buffer allocator returned no value".to_owned()))
}

#[allow(clippy::too_many_arguments)]
fn lower_complex_expression(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    match expression {
        Expr::Try { expression, span } => lower_try_expression(
            function,
            expression,
            *span,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
        Expr::Unary { .. } | Expr::Binary { .. } => lower_operation(
            function,
            expression,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
        _ => lower_construct_or_case(
            function,
            expression,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_construct_or_case(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    match expression {
        Expr::Call { .. }
        | Expr::MethodCall { .. }
        | Expr::StructLit { .. }
        | Expr::FieldAccess { .. } => lower_construct(
            function,
            expression,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
        Expr::Case { subject, branches, .. } => lower_case(
            function,
            subject,
            branches,
            locals,
            local_types,
            functions,
            cleanup_schedule,
            string_data,
            layouts,
        ),
        _ => Err(NativeEmitError("unsupported native expression".to_owned())),
    }
}

#[allow(clippy::too_many_arguments)]
fn lower_try_expression(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    span: crate::lexer::SourceSpan,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    string_data: &StringDataValues,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let source = lower_expression(
        function,
        expression,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        string_data,
        layouts,
    )?;
    let NativeType::Enum(enum_id) = initializer_type(expression, local_types, functions, layouts)
    else {
        return Err(NativeEmitError("try operand is not a native Result value".to_owned()));
    };
    let layout = layouts
        .enum_layout(enum_id)
        .ok_or_else(|| NativeEmitError("try operand has no enum layout".to_owned()))?;
    let ok = layouts
        .enum_variant(enum_id, "Ok")
        .ok_or_else(|| NativeEmitError("Result enum has no Ok variant".to_owned()))?;
    let field =
        ok.fields.first().ok_or_else(|| NativeEmitError("Result.Ok has no payload".to_owned()))?;
    emit_try_control_flow(
        function,
        source,
        span,
        field.ty,
        layout.discriminant_offset,
        layout.payload_offset + field.offset,
        ok.discriminant,
        locals,
        local_types,
        functions,
        cleanup_schedule,
        layouts,
    )
}

#[allow(clippy::too_many_arguments)]
fn emit_try_control_flow(
    function: &mut FunctionBuilder<'_>,
    source: cranelift_codegen::ir::Value,
    span: crate::lexer::SourceSpan,
    field_type: NativeType,
    discriminant_offset: u32,
    payload_offset: u32,
    ok_discriminant: u32,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    local_types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    cleanup_schedule: &NativeCleanupSchedule,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let discriminant =
        function.ins().load(types::I32, MemFlagsData::new(), source, discriminant_offset as i32);
    let is_ok = function.ins().icmp_imm_s(IntCC::Equal, discriminant, i64::from(ok_discriminant));
    let ok_block = function.create_block();
    let err_block = function.create_block();
    let merge = function.create_block();
    function.append_block_param(merge, try_payload_ir_type(field_type, layouts));
    function.ins().brif(is_ok, ok_block, &[], err_block, &[]);

    function.switch_to_block(err_block);
    emit_return_cleanup(function, cleanup_schedule, span, locals, local_types, functions, layouts)?;
    function.ins().return_(&[source]);
    function.seal_block(err_block);

    function.switch_to_block(ok_block);
    let payload_address = function.ins().iadd_imm_s(source, i64::from(payload_offset));
    let value = load_try_payload(function, payload_address, field_type, layouts)?;
    let argument = cranelift_codegen::ir::BlockArg::Value(value);
    function.ins().jump(merge, [&argument]);
    function.seal_block(ok_block);

    function.switch_to_block(merge);
    function.seal_block(merge);
    Ok(function.block_params(merge)[0])
}

fn load_try_payload(
    function: &mut FunctionBuilder<'_>,
    payload_address: cranelift_codegen::ir::Value,
    field_type: NativeType,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    match field_type {
        NativeType::Struct(_) | NativeType::Enum(_) => Ok(payload_address),
        NativeType::Int => {
            Ok(function.ins().load(types::I32, MemFlagsData::new(), payload_address, 0))
        }
        NativeType::Integer { width, .. } => Ok(function.ins().load(
            NativeType::Integer { signed: false, width }.ir_type(layouts.pointer_type),
            MemFlagsData::new(),
            payload_address,
            0,
        )),
        NativeType::Float { width } => Ok(function.ins().load(
            NativeType::Float { width }.ir_type(layouts.pointer_type),
            MemFlagsData::new(),
            payload_address,
            0,
        )),
        NativeType::Void => Ok(function.ins().iconst(types::I32, 0)),
        NativeType::String | NativeType::Buffer => {
            Ok(function.ins().load(layouts.pointer_type, MemFlagsData::new(), payload_address, 0))
        }
        NativeType::FatPointer => {
            Err(NativeEmitError("try does not yet unwrap fat-pointer payloads".to_owned()))
        }
    }
}

fn try_payload_ir_type(
    field_type: NativeType,
    layouts: &LayoutRegistry,
) -> cranelift_codegen::ir::Type {
    if matches!(field_type, NativeType::Void) {
        types::I32
    } else {
        field_type.ir_type(layouts.pointer_type)
    }
}

pub(super) fn initializer_type(
    expression: &Expr,
    types: &HashMap<&String, NativeType>,
    functions: &HashMap<String, FunctionRef>,
    layouts: &LayoutRegistry,
) -> NativeType {
    match expression {
        Expr::Identifier { name, .. } => types.get(name).copied().unwrap_or(NativeType::Int),
        Expr::BufferLiteral { .. } => NativeType::Buffer,
        Expr::Grouping { expression, .. } | Expr::Borrow { expression, .. } => {
            initializer_type(expression, types, functions, layouts)
        }
        Expr::Try { expression, .. } => {
            let NativeType::Enum(enum_id) = initializer_type(expression, types, functions, layouts)
            else {
                return NativeType::Int;
            };
            layouts
                .enum_variant(enum_id, "Ok")
                .and_then(|variant| variant.fields.first().map(|field| field.ty))
                .unwrap_or(NativeType::Int)
        }
        Expr::Call { callee, .. } => {
            functions.get(callee).map(|function| function.return_type).unwrap_or(NativeType::Int)
        }
        Expr::MethodCall { receiver, method, .. } => {
            let receiver_type = initializer_type(receiver, types, functions, layouts);
            let dispatch_name = super::performance::dispatch_key(receiver_type, method);
            enum_expression_type(expression, layouts)
                .or_else(|| functions.get(&dispatch_name).map(|function| function.return_type))
                .or_else(|| functions.get(method).map(|function| function.return_type))
                .unwrap_or(NativeType::Int)
        }
        Expr::StructLit { name, type_arguments, .. } => {
            let type_name = crate::ast::TypeName {
                name: name.clone(),
                arguments: type_arguments.clone(),
                span: crate::lexer::SourceSpan::new(0, 0),
            };
            layouts.type_for_type_name(&type_name).unwrap_or(NativeType::Int)
        }
        Expr::FieldAccess { object, field, .. } => expression_native_type(object, types, layouts)
            .and_then(|ty| field_type(ty, field, layouts))
            .or_else(|| enum_expression_type(expression, layouts))
            .unwrap_or(NativeType::Int),
        Expr::FloatLiteral { .. } => NativeType::Int,
        _ => NativeType::Int,
    }
}

pub(super) fn emit_buffer_drop(
    function: &mut FunctionBuilder<'_>,
    name: &str,
    locals: &HashMap<&String, cranelift_codegen::ir::Value>,
    functions: &HashMap<String, FunctionRef>,
) -> Result<(), NativeEmitError> {
    let value = locals
        .iter()
        .find(|(binding, _)| binding.as_str() == name)
        .map(|(_, value)| *value)
        .ok_or_else(|| NativeEmitError(format!("native binding `{name}` is unavailable")))?;
    let target = functions.get("actus_buffer_drop").ok_or_else(|| {
        NativeEmitError("native runtime function `actus_buffer_drop` is unavailable".to_owned())
    })?;
    function.ins().call(target.reference, &[value]);
    Ok(())
}
