use cranelift_codegen::ir::{InstBuilder, TrapCode, Value, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;

use crate::ast::{Expr, TypeName};

use super::super::calls::CallLoweringContext;
use super::super::native::NativeEmitError;
use super::super::structs::expression_native_type;
use super::super::types::NativeType;
use super::lower_expression;

pub(super) fn lower_cast(
    function: &mut FunctionBuilder<'_>,
    expression: &Expr,
    target: &TypeName,
    context: &CallLoweringContext<'_, '_>,
) -> Result<Value, NativeEmitError> {
    let source = lower_expression(
        function,
        expression,
        context.locals,
        context.local_types,
        context.functions,
        context.cleanup_schedule,
        context.string_data,
        context.layouts,
    )?;
    let source_type = expression_native_type(expression, context.local_types, context.layouts)
        .unwrap_or(NativeType::Int);
    let target_type = NativeType::from_type_name_with_layout(Some(target), context.layouts)?;
    let target_ir = context.layouts.ir_type(target_type)?;
    let checked = checked_integer_value(function, source, source_type, target_type)?;
    Ok(convert_integer(function, checked, target_ir, source_type, target_type))
}

fn checked_integer_value(
    function: &mut FunctionBuilder<'_>,
    value: Value,
    source: NativeType,
    target: NativeType,
) -> Result<Value, NativeEmitError> {
    let (target_signed, target_width) = integer_shape(target)?;
    let (source_signed, source_width) = integer_shape(source)?;
    let wide = widen_to_i64(function, value, source_signed);
    let valid =
        cast_validity(function, wide, source_signed, source_width, target_signed, target_width);
    let ok_block = function.create_block();
    let trap_block = function.create_block();
    function.ins().brif(valid, ok_block, &[], trap_block, &[]);
    function.switch_to_block(trap_block);
    function.ins().trap(TrapCode::INTEGER_OVERFLOW);
    function.seal_block(trap_block);
    function.switch_to_block(ok_block);
    function.seal_block(ok_block);
    Ok(wide)
}

fn cast_validity(
    function: &mut FunctionBuilder<'_>,
    value: Value,
    source_signed: bool,
    source_width: u8,
    target_signed: bool,
    target_width: u8,
) -> Value {
    let source_bits = i64::from(source_width);
    if target_width >= source_width && source_signed == target_signed {
        return function.ins().iconst(types::I8, 1);
    }
    let zero = function.ins().iconst(types::I64, 0);
    let lower = if !source_signed {
        function.ins().iconst(types::I8, 1)
    } else if target_signed {
        let minimum = if target_width == 64 { i64::MIN } else { -(1i64 << (target_width - 1)) };
        let limit = function.ins().iconst(types::I64, minimum);
        function.ins().icmp(IntCC::SignedGreaterThanOrEqual, value, limit)
    } else {
        function.ins().icmp(IntCC::SignedGreaterThanOrEqual, value, zero)
    };
    let upper = if target_width == 64 {
        if target_signed && source_signed { lower } else { function.ins().iconst(types::I8, 1) }
    } else {
        let maximum = if target_signed {
            (1i64 << (target_width - 1)) - 1
        } else {
            (1i64 << target_width) - 1
        };
        let condition = if source_signed {
            IntCC::SignedLessThanOrEqual
        } else {
            IntCC::UnsignedLessThanOrEqual
        };
        let limit = function.ins().iconst(types::I64, maximum);
        function.ins().icmp(condition, value, limit)
    };
    if i64::from(target_width) >= source_bits && !source_signed {
        upper
    } else {
        function.ins().band(lower, upper)
    }
}

fn widen_to_i64(function: &mut FunctionBuilder<'_>, value: Value, signed: bool) -> Value {
    let source = function.func.dfg.value_type(value);
    if source == types::I64 {
        return value;
    }
    if signed {
        function.ins().sextend(types::I64, value)
    } else {
        function.ins().uextend(types::I64, value)
    }
}

fn convert_integer(
    function: &mut FunctionBuilder<'_>,
    value: Value,
    target_ir: cranelift_codegen::ir::Type,
    _source: NativeType,
    _target: NativeType,
) -> Value {
    if target_ir == types::I64 { value } else { function.ins().ireduce(target_ir, value) }
}

fn integer_shape(ty: NativeType) -> Result<(bool, u8), NativeEmitError> {
    match ty {
        NativeType::Int => Ok((true, 32)),
        NativeType::Integer { signed, width } => Ok((signed, width)),
        other => Err(NativeEmitError(format!("cannot cast non-integer native type {other:?}"))),
    }
}
