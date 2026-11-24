use cranelift_codegen::ir::InstBuilder;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_frontend::FunctionBuilder;

use super::narrow::{integer_bounds, narrow_overflow, narrow_width};
use crate::codegen::{BackendIntegerBinaryOp, BackendValueType};

/// The wrapped result of an add, subtract or multiply and whether the exact
/// result overflowed the operand type (ADR-0115).
pub(in crate::codegen::cranelift) fn overflowing(
    builder: &mut FunctionBuilder<'_>,
    operation: BackendIntegerBinaryOp,
    signed: bool,
    left: cranelift_codegen::ir::Value,
    right: cranelift_codegen::ir::Value,
) -> (cranelift_codegen::ir::Value, cranelift_codegen::ir::Value) {
    use BackendIntegerBinaryOp::{
        SaturatingAdd, SaturatingSubtract, TrappingAdd, TrappingSubtract,
    };
    let instructions = builder.ins();
    match (operation, signed) {
        (TrappingAdd | SaturatingAdd, true) => instructions.sadd_overflow(left, right),
        (TrappingAdd | SaturatingAdd, false) => instructions.uadd_overflow(left, right),
        (TrappingSubtract | SaturatingSubtract, true) => instructions.ssub_overflow(left, right),
        (TrappingSubtract | SaturatingSubtract, false) => instructions.usub_overflow(left, right),
        (_, true) => instructions.smul_overflow(left, right),
        (_, false) => instructions.umul_overflow(left, right),
    }
}

/// Like [`overflowing`] for an operand type that may be narrower than its
/// machine carrier: a result outside the type's own range also overflows.
pub(in crate::codegen::cranelift) fn overflowing_in(
    builder: &mut FunctionBuilder<'_>,
    operation: BackendIntegerBinaryOp,
    ty: BackendValueType,
    left: cranelift_codegen::ir::Value,
    right: cranelift_codegen::ir::Value,
) -> (cranelift_codegen::ir::Value, cranelift_codegen::ir::Value) {
    let signed = matches!(ty, BackendValueType::Integer { signed: true, .. });
    let (value, overflow) = overflowing(builder, operation, signed, left, right);
    let carrier = builder.func.dfg.value_type(value);
    if narrow_width(ty, carrier).is_none() {
        return (value, overflow);
    }
    (value, narrow_overflow(builder, value, overflow, ty))
}

/// The result clamped to the bound the exact result crossed: the maximum,
/// or for a signed operation whose exact result is negative, the minimum;
/// an unsigned subtraction clamps at zero.
pub(in crate::codegen::cranelift) fn saturating(
    builder: &mut FunctionBuilder<'_>,
    operation: BackendIntegerBinaryOp,
    signed: bool,
    left: cranelift_codegen::ir::Value,
    right: cranelift_codegen::ir::Value,
) -> cranelift_codegen::ir::Value {
    let width = builder.func.dfg.value_type(left).bits() as u8;
    saturating_in(builder, operation, BackendValueType::Integer { signed, width }, left, right)
}

/// [`saturating`] for the full range of `ty`, which may be narrower than its
/// machine carrier.
pub(in crate::codegen::cranelift) fn saturating_in(
    builder: &mut FunctionBuilder<'_>,
    operation: BackendIntegerBinaryOp,
    ty: BackendValueType,
    left: cranelift_codegen::ir::Value,
    right: cranelift_codegen::ir::Value,
) -> cranelift_codegen::ir::Value {
    let BackendValueType::Integer { signed, width } = ty else {
        unreachable!("saturating arithmetic requires an integer type")
    };
    let (value, overflow) = overflowing_in(builder, operation, ty, left, right);
    let carrier = builder.func.dfg.value_type(value);
    let (minimum, maximum) = integer_bounds(builder, carrier, signed, width);
    let bound = if !signed {
        if operation == BackendIntegerBinaryOp::SaturatingSubtract {
            builder.ins().iconst(carrier, 0)
        } else {
            maximum
        }
    } else {
        let sign_source = match operation {
            BackendIntegerBinaryOp::SaturatingMultiply => builder.ins().bxor(left, right),
            _ => right,
        };
        let zero = builder.ins().iconst(carrier, 0);
        let negative = builder.ins().icmp(IntCC::SignedLessThan, sign_source, zero);
        match operation {
            BackendIntegerBinaryOp::SaturatingSubtract => {
                builder.ins().select(negative, maximum, minimum)
            }
            _ => builder.ins().select(negative, minimum, maximum),
        }
    };
    builder.ins().select(overflow, bound, value)
}
