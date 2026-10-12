// Packed expression lowering receives the complete neutral lowering context.
#![allow(clippy::too_many_arguments)]

use super::super::super::{
    BackendContractError, BackendInstruction, BackendIntegerBinaryOp, BackendValueId,
    BackendValueType,
};
use super::super::SharedExpressionProvider;
use super::super::support::unsupported;
use crate::ast::{CompoundAssignmentOp, Expr, TypeName};
use crate::codegen::body::BackendExpressionProvider;
use crate::lexer::SourceSpan;

pub(super) fn field_type(
    metadata: &super::super::super::BackendPackedFieldLayout,
) -> Option<BackendValueType> {
    let primitive = crate::ast::primitive_type(&metadata.ty.name)?;
    let crate::ast::PrimitiveType::Integer { signed, width } = primitive else { return None };
    let width = match width {
        1..=32 => 32,
        33..=64 => 64,
        _ => return None,
    };
    Some(BackendValueType::Integer { signed, width })
}

pub(super) fn lower_field_access(
    provider: &mut SharedExpressionProvider<'_>,
    base: BackendValueId,
    type_name: &TypeName,
    field: &str,
    aggregate_layout: super::super::super::BackendLayout,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let metadata = provider
        .layouts
        .packed_field_for(type_name, field)
        .ok_or_else(|| unsupported(span, "packed field metadata is unavailable"))?;
    let ty = field_type(&metadata)
        .ok_or_else(|| unsupported(span, "packed field type has no neutral scalar type"))?;
    let result = provider.allocate();
    provider.remember_value(result, ty);
    instructions.push(BackendInstruction::PackedFieldLoad {
        result,
        base,
        aggregate_layout,
        storage_layout: metadata.storage,
        bit_offset: metadata.bit_offset,
        width: metadata.width,
        big_endian: metadata.big_endian,
        ty,
        span,
    });
    Ok(result)
}

pub(super) fn lower_field_assignment(
    provider: &mut SharedExpressionProvider<'_>,
    base: BackendValueId,
    type_name: &TypeName,
    field: &str,
    value: &Expr,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(), BackendContractError> {
    let metadata = provider
        .layouts
        .packed_field_for(type_name, field)
        .ok_or_else(|| unsupported(span, "packed field metadata is unavailable"))?;
    let ty = field_type(&metadata)
        .ok_or_else(|| unsupported(span, "packed field type has no neutral scalar type"))?;
    let value = provider.lower_expression(value, instructions)?;
    let value = super::state::coerce_value_to_type(provider, value, Some(ty), span, instructions)?;
    let aggregate_layout = provider
        .layouts
        .layout_for(type_name)
        .ok_or_else(|| unsupported(span, "aggregate base has no neutral layout"))?;
    instructions.push(BackendInstruction::PackedFieldStore {
        base,
        aggregate_layout,
        storage_layout: metadata.storage,
        bit_offset: metadata.bit_offset,
        width: metadata.width,
        big_endian: metadata.big_endian,
        value,
        ty,
        span,
    });
    Ok(())
}

pub(super) fn lower_field_compound_assignment(
    provider: &mut SharedExpressionProvider<'_>,
    base: BackendValueId,
    type_name: &TypeName,
    field: &str,
    operator: CompoundAssignmentOp,
    value: &Expr,
    aggregate_layout: super::super::super::BackendLayout,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(), BackendContractError> {
    let metadata = provider
        .layouts
        .packed_field_for(type_name, field)
        .ok_or_else(|| unsupported(span, "packed field metadata is unavailable"))?;
    let ty = field_type(&metadata)
        .ok_or_else(|| unsupported(span, "packed field type has no neutral scalar type"))?;
    let current = provider.allocate();
    provider.remember_value(current, ty);
    instructions.push(BackendInstruction::PackedFieldLoad {
        result: current,
        base,
        aggregate_layout,
        storage_layout: metadata.storage,
        bit_offset: metadata.bit_offset,
        width: metadata.width,
        big_endian: metadata.big_endian,
        ty,
        span,
    });
    let value = provider.lower_expression(value, instructions)?;
    let value = super::state::coerce_value_to_type(provider, value, Some(ty), span, instructions)?;
    let result = provider.lower_integer_binary(
        compound_operation(operator),
        current,
        value,
        span,
        instructions,
    )?;
    instructions.push(BackendInstruction::PackedFieldStore {
        base,
        aggregate_layout,
        storage_layout: metadata.storage,
        bit_offset: metadata.bit_offset,
        width: metadata.width,
        big_endian: metadata.big_endian,
        value: result,
        ty,
        span,
    });
    Ok(())
}

fn compound_operation(operator: CompoundAssignmentOp) -> BackendIntegerBinaryOp {
    match operator {
        CompoundAssignmentOp::Add => BackendIntegerBinaryOp::Add,
        CompoundAssignmentOp::Subtract => BackendIntegerBinaryOp::Subtract,
        CompoundAssignmentOp::Multiply => BackendIntegerBinaryOp::Multiply,
        CompoundAssignmentOp::Divide => BackendIntegerBinaryOp::Divide,
        CompoundAssignmentOp::Remainder => BackendIntegerBinaryOp::Remainder,
        CompoundAssignmentOp::ShiftLeft => BackendIntegerBinaryOp::ShiftLeft,
        CompoundAssignmentOp::ShiftRight => BackendIntegerBinaryOp::ShiftRight,
        CompoundAssignmentOp::BitwiseAnd => BackendIntegerBinaryOp::BitwiseAnd,
        CompoundAssignmentOp::BitwiseOr => BackendIntegerBinaryOp::BitwiseOr,
        CompoundAssignmentOp::BitwiseXor => BackendIntegerBinaryOp::BitwiseXor,
    }
}
