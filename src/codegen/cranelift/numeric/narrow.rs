//! Integers whose width is not a machine width (`i33`, `u40`, `i63`, ...).
//!
//! A value of width `w` lives in the smallest machine integer that holds it
//! (the carrier) and is kept canonical: sign-extended from bit `w - 1` for a
//! signed type, zero-extended for an unsigned one. Canonical values compare
//! and divide exactly like their carrier; only operations that can leave the
//! width range (arithmetic, shifts, negation, complement) must re-establish
//! the invariant or detect that they could not.

use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{InstBuilder, Type, Value, types};
use cranelift_frontend::FunctionBuilder;

use crate::codegen::BackendValueType;

/// The machine integer that carries a value of `width` bits (1 to 64).
pub(crate) fn integer_carrier_type(width: u8) -> Option<Type> {
    match width {
        1..=8 => Some(types::I8),
        9..=16 => Some(types::I16),
        17..=32 => Some(types::I32),
        33..=64 => Some(types::I64),
        _ => None,
    }
}

/// The width of a type that does not fill its carrier.
pub(super) fn narrow_width(ty: BackendValueType, carrier: Type) -> Option<u8> {
    match ty {
        BackendValueType::Integer { width, .. } if u32::from(width) < carrier.bits() => Some(width),
        _ => None,
    }
}

/// `value` re-extended from its low `width` bits.
pub(super) fn canonicalize(
    builder: &mut FunctionBuilder<'_>,
    value: Value,
    signed: bool,
    width: u8,
) -> Value {
    let carrier = builder.func.dfg.value_type(value);
    let shift = i64::from(carrier.bits() - u32::from(width));
    let amount = builder.ins().iconst(carrier, shift);
    let raised = builder.ins().ishl(value, amount);
    if signed { builder.ins().sshr(raised, amount) } else { builder.ins().ushr(raised, amount) }
}

/// `value` canonicalized when `ty` is narrower than its carrier.
pub(super) fn canonicalize_for(
    builder: &mut FunctionBuilder<'_>,
    value: Value,
    ty: BackendValueType,
) -> Value {
    let carrier = builder.func.dfg.value_type(value);
    match (narrow_width(ty, carrier), ty) {
        (Some(width), BackendValueType::Integer { signed, .. }) => {
            canonicalize(builder, value, signed, width)
        }
        _ => value,
    }
}

/// `overflow` widened to also report a result outside the width of `ty`.
pub(super) fn narrow_overflow(
    builder: &mut FunctionBuilder<'_>,
    value: Value,
    overflow: Value,
    ty: BackendValueType,
) -> Value {
    let canonical = canonicalize_for(builder, value, ty);
    let outside = builder.ins().icmp(IntCC::NotEqual, canonical, value);
    builder.ins().bor(overflow, outside)
}

/// The smallest and largest value of an integer type, as carrier constants.
pub(super) fn integer_bounds(
    builder: &mut FunctionBuilder<'_>,
    carrier: Type,
    signed: bool,
    width: u8,
) -> (Value, Value) {
    let (minimum, maximum) = if signed {
        (i64::MIN >> (64 - u32::from(width)), i64::MAX >> (64 - u32::from(width)))
    } else {
        (0, (u64::MAX >> (64 - u32::from(width))) as i64)
    };
    (builder.ins().iconst(carrier, minimum), builder.ins().iconst(carrier, maximum))
}
