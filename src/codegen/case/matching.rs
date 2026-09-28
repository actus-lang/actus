use cranelift_codegen::ir::{InstBuilder, MemFlagsData, condcodes::IntCC, types};
use cranelift_frontend::FunctionBuilder;

use crate::ast::Pattern;

use super::super::layout::LayoutRegistry;
use super::super::native::NativeEmitError;
use super::super::types::NativeType;

pub(super) fn match_pattern(
    function: &mut FunctionBuilder<'_>,
    subject: cranelift_codegen::ir::Value,
    subject_type: NativeType,
    pattern: &Pattern,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    match pattern {
        Pattern::Wildcard { .. } => Ok(function.ins().iconst(types::I8, 1)),
        Pattern::Literal { value, .. } => {
            let expected = match value {
                crate::ast::LiteralPattern::Integer(value) => parse_integer_pattern(value)?,
                crate::ast::LiteralPattern::Bool(value) => i64::from(*value),
            };
            Ok(function.ins().icmp_imm_s(IntCC::Equal, subject, expected))
        }
        Pattern::Variant { enum_name, variant, .. } => {
            match_variant_pattern(function, subject, subject_type, enum_name, variant, layouts)
        }
    }
}

fn match_variant_pattern(
    function: &mut FunctionBuilder<'_>,
    subject: cranelift_codegen::ir::Value,
    subject_type: NativeType,
    enum_name: &str,
    variant: &str,
    layouts: &LayoutRegistry,
) -> Result<cranelift_codegen::ir::Value, NativeEmitError> {
    let NativeType::Enum(enum_id) = subject_type else {
        return Err(NativeEmitError(format!("case subject is not enum `{enum_name}`")));
    };
    let enum_layout = layouts
        .enum_layout(enum_id)
        .ok_or_else(|| NativeEmitError("missing enum layout for case subject".to_owned()))?;
    let variant_layout = layouts
        .enum_variant(enum_id, variant)
        .ok_or_else(|| NativeEmitError(format!("unknown case variant `{enum_name}.{variant}`")))?;
    Ok(variant_match_value(function, subject, variant, enum_layout, variant_layout))
}

fn variant_match_value(
    function: &mut FunctionBuilder<'_>,
    subject: cranelift_codegen::ir::Value,
    variant: &str,
    enum_layout: &super::super::enum_layout::EnumLayout,
    variant_layout: &super::super::enum_layout::EnumVariantLayout,
) -> cranelift_codegen::ir::Value {
    if enum_layout.niche_pointer {
        let comparison = if variant == "None" { IntCC::Equal } else { IntCC::NotEqual };
        return function.ins().icmp_imm_s(comparison, subject, 0);
    }
    let discriminant = function.ins().load(
        types::I32,
        MemFlagsData::new(),
        subject,
        enum_layout.discriminant_offset as i32,
    );
    function.ins().icmp_imm_s(IntCC::Equal, discriminant, i64::from(variant_layout.discriminant))
}

fn parse_integer_pattern(value: &str) -> Result<i64, NativeEmitError> {
    let (negative, digits) =
        value.strip_prefix('-').map_or((false, value), |digits| (true, digits));
    let magnitude = digits
        .strip_prefix("0x")
        .or_else(|| digits.strip_prefix("0X"))
        .map_or_else(|| digits.parse::<i128>(), |hex| i128::from_str_radix(hex, 16))
        .map_err(|_| NativeEmitError(format!("invalid integer pattern `{value}`")))?;
    let signed = if negative { magnitude.checked_neg() } else { Some(magnitude) }
        .ok_or_else(|| NativeEmitError(format!("integer pattern `{value}` overflows")))?;
    i64::try_from(signed)
        .map_err(|_| NativeEmitError(format!("integer pattern `{value}` overflows")))
}
