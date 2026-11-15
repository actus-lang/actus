use super::{arrays, compound, fields, metadata, structs};
use crate::ast::{Expr, TypeName};
use crate::codegen::{BackendContractError, BackendInstruction, BackendValueId};

use super::super::super::SharedExpressionProvider;
use super::super::super::errors::{expression_span, unsupported};

type ArrayElementLocationParts = (
    BackendValueId,
    BackendValueId,
    crate::codegen::BackendLayout,
    crate::codegen::BackendLayout,
    TypeName,
    crate::codegen::BackendValueType,
    u32,
    bool,
);

pub(in crate::codegen::shared_lowering::expression) fn parameter_nominal_types(
    verb: &crate::ast::VerbDecl,
    layouts: &dyn crate::codegen::BackendLayoutProvider,
) -> std::collections::BTreeMap<BackendValueId, TypeName> {
    metadata::parameter_nominal_types(verb, layouts)
}
pub(in crate::codegen::shared_lowering::expression) fn lower_array_element(
    provider: &mut SharedExpressionProvider<'_>,
    collection: &Expr,
    index: &Expr,
    span: crate::lexer::SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    arrays::lower_array_element(provider, collection, index, span, instructions)
}
pub(in crate::codegen::shared_lowering::expression) fn lower_array_element_assignment(
    provider: &mut SharedExpressionProvider<'_>,
    collection: &Expr,
    index: &Expr,
    value: &Expr,
    span: crate::lexer::SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(), BackendContractError> {
    arrays::lower_array_element_assignment(provider, collection, index, value, span, instructions)
}

pub(in crate::codegen::shared_lowering::expression) fn lower_array_element_location(
    provider: &mut SharedExpressionProvider<'_>,
    collection: &Expr,
    index: &Expr,
    span: crate::lexer::SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<ArrayElementLocationParts, BackendContractError> {
    let location =
        arrays::lower_array_element_location(provider, collection, index, span, instructions)?;
    Ok((
        location.base,
        location.index,
        location.aggregate_layout,
        location.element_layout,
        location.element_type_name,
        location.element_type,
        location.capacity,
        location.big_endian,
    ))
}

pub(in crate::codegen::shared_lowering::expression) fn lower_field_assignment(
    provider: &mut SharedExpressionProvider<'_>,
    object: &Expr,
    field: &str,
    value: &Expr,
    span: crate::lexer::SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(), BackendContractError> {
    fields::lower_field_assignment(provider, object, field, value, span, instructions)
}

pub(in crate::codegen::shared_lowering::expression) fn lower_field_compound_assignment(
    provider: &mut SharedExpressionProvider<'_>,
    object: &Expr,
    field: &str,
    operator: crate::ast::CompoundAssignmentOp,
    value: &Expr,
    span: crate::lexer::SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(), BackendContractError> {
    compound::lower_field_compound_assignment(
        provider,
        object,
        field,
        operator,
        value,
        span,
        instructions,
    )
}

pub(in crate::codegen::shared_lowering::expression) fn lower_aggregate_expression(
    provider: &mut SharedExpressionProvider<'_>,
    expression: &Expr,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    match expression {
        Expr::StructLit { .. } => {
            structs::lower_struct_literal_expression(provider, expression, instructions)
        }
        Expr::FieldAccess { object, field, resolved_type, span } => {
            if let Some(type_name) = resolved_type
                && provider.enum_variant(type_name, field).is_some()
            {
                return super::super::super::constructors::lower_named_unit_enum(
                    provider,
                    type_name,
                    field,
                    &[],
                    *span,
                    instructions,
                );
            }
            fields::lower_field_access_expression(provider, object, field, *span, instructions)
        }
        Expr::Index { target, index, span } => {
            arrays::lower_array_element(provider, target, index, *span, instructions)
        }
        _ => Err(unsupported(expression_span(expression), "aggregate expression expected")),
    }
}
