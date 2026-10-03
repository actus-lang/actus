use cranelift_codegen::ir::{InstBuilder, MemFlagsData, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;

use crate::ast::Expr;
use crate::lexer::SourceSpan;

use super::super::calls::CallLoweringContext;
use super::super::cleanup::emit_return_cleanup;
use super::super::layout::LayoutRegistry;
use super::super::native::NativeEmitError;
use super::super::types::NativeType;
use super::initializer_types::initializer_type;

struct TryPayloadLayout {
    field_type: NativeType,
    discriminant_offset: u32,
    payload_offset: u32,
    ok_discriminant: u32,
}

pub(crate) fn lower_try_expression(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    span: SourceSpan,
    context: &CallLoweringContext<'_, '_>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let source = lower_try_source(function, expression, context)?;
    let NativeType::Enum(enum_id) =
        initializer_type(expression, context.local_types, context.functions, context.layouts)?
    else {
        return Err(NativeEmitError("try operand is not a native Result value".to_owned()));
    };
    let layout = context
        .layouts
        .enum_layout(enum_id)
        .ok_or_else(|| NativeEmitError("try operand has no enum layout".to_owned()))?;
    let ok = context
        .layouts
        .enum_variant(enum_id, "Ok")
        .ok_or_else(|| NativeEmitError("Result enum has no Ok variant".to_owned()))?;
    let field =
        ok.fields.first().ok_or_else(|| NativeEmitError("Result.Ok has no payload".to_owned()))?;
    let payload_layout = TryPayloadLayout {
        field_type: field.ty,
        discriminant_offset: layout.discriminant_offset,
        payload_offset: layout.payload_offset + field.offset,
        ok_discriminant: ok.discriminant,
    };
    emit_try_control_flow(function, source, span, payload_layout, context)
}

fn lower_try_source(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    context: &CallLoweringContext<'_, '_>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    super::lower_expression(
        function,
        expression,
        context.locals,
        context.local_types,
        context.functions,
        context.cleanup_schedule,
        context.string_data,
        context.layouts,
    )
}

fn emit_try_control_flow(
    function: &mut FunctionBuilder<'_>,
    source: cranelift_codegen::ir::Value,
    span: SourceSpan,
    payload_layout: TryPayloadLayout,
    context: &CallLoweringContext<'_, '_>,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let is_ok = try_is_ok(function, source, &payload_layout);
    let ok_block = function.create_block();
    let err_block = function.create_block();
    let merge = function.create_block();
    function.append_block_param(
        merge,
        try_payload_ir_type(payload_layout.field_type, context.layouts)?,
    );
    function.ins().brif(is_ok, ok_block, &[], err_block, &[]);

    function.switch_to_block(err_block);
    emit_try_error_return(function, source, span, context)?;
    function.seal_block(err_block);

    emit_try_success(function, source, ok_block, merge, payload_layout, context)?;

    function.switch_to_block(merge);
    function.seal_block(merge);
    Ok(function.block_params(merge)[0])
}

fn try_is_ok(
    function: &mut FunctionBuilder<'_>,
    source: cranelift_codegen::ir::Value,
    payload_layout: &TryPayloadLayout,
) -> cranelift_codegen::ir::Value {
    let discriminant = function.ins().load(
        types::I32,
        MemFlagsData::new(),
        source,
        payload_layout.discriminant_offset as i32,
    );
    function.ins().icmp_imm_s(IntCC::Equal, discriminant, i64::from(payload_layout.ok_discriminant))
}

fn emit_try_success(
    function: &mut FunctionBuilder<'_>,
    source: cranelift_codegen::ir::Value,
    ok_block: cranelift_codegen::ir::Block,
    merge: cranelift_codegen::ir::Block,
    payload_layout: TryPayloadLayout,
    context: &CallLoweringContext<'_, '_>,
) -> Result<(), NativeEmitError> {
    function.switch_to_block(ok_block);
    let payload_address =
        function.ins().iadd_imm_s(source, i64::from(payload_layout.payload_offset));
    let value =
        load_try_payload(function, payload_address, payload_layout.field_type, context.layouts)?;
    let argument = cranelift_codegen::ir::BlockArg::Value(value);
    function.ins().jump(merge, [&argument]);
    function.seal_block(ok_block);
    Ok(())
}

fn emit_try_error_return(
    function: &mut FunctionBuilder<'_>,
    source: cranelift_codegen::ir::Value,
    span: SourceSpan,
    context: &CallLoweringContext<'_, '_>,
) -> Result<(), NativeEmitError> {
    emit_return_cleanup(
        function,
        context.cleanup_schedule,
        span,
        context.locals,
        context.local_types,
        context.functions,
        context.layouts,
        None,
    )?;
    function.ins().return_(&[source]);
    Ok(())
}

fn load_try_payload(
    function: &mut FunctionBuilder<'_>,
    payload_address: cranelift_codegen::ir::Value,
    field_type: NativeType,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    match field_type {
        NativeType::Struct(_)
        | NativeType::Enum(_)
        | NativeType::Array(_)
        | NativeType::Arena(_) => Ok(payload_address),
        NativeType::Int => Ok(load_payload(function, payload_address, types::I32)),
        NativeType::Integer { width, .. } => {
            let ty = layouts.ir_type(NativeType::Integer { signed: false, width })?;
            Ok(load_payload(function, payload_address, ty))
        }
        NativeType::Pack(id) => load_pack_payload(function, payload_address, id, layouts),
        NativeType::Float { width } => {
            let ty = layouts.ir_type(NativeType::Float { width })?;
            Ok(load_payload(function, payload_address, ty))
        }
        NativeType::Void => Ok(function.ins().iconst(types::I32, 0)),
        NativeType::String | NativeType::Buffer => {
            Ok(load_payload(function, payload_address, layouts.pointer_type))
        }
        NativeType::FatPointer => {
            Err(NativeEmitError("try does not yet unwrap fat-pointer payloads".to_owned()))
        }
    }
}

fn load_payload(
    function: &mut FunctionBuilder<'_>,
    address: cranelift_codegen::ir::Value,
    ty: cranelift_codegen::ir::Type,
) -> cranelift_codegen::ir::Value {
    function.ins().load(ty, MemFlagsData::new(), address, 0)
}

fn load_pack_payload(
    function: &mut FunctionBuilder<'_>,
    address: cranelift_codegen::ir::Value,
    id: usize,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let pack = layouts
        .pack(id)
        .ok_or_else(|| NativeEmitError("missing packed payload layout".to_owned()))?;
    let ty = layouts.ir_type(pack.storage)?;
    Ok(load_payload(function, address, ty))
}

fn try_payload_ir_type(
    field_type: NativeType,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Type, NativeEmitError> {
    if matches!(field_type, NativeType::Void) {
        Ok(types::I32)
    } else {
        layouts.ir_type(field_type)
    }
}
