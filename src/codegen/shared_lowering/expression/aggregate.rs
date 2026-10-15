mod arrays;
mod compound;
mod fields;
mod metadata;
mod structs;

use crate::ast::{Expr, TypeName};
use crate::codegen::{BackendContractError, BackendInstruction, BackendValueId};

use super::super::SharedExpressionProvider;
use super::super::support::{expression_span, unsupported};

pub(super) fn parameter_nominal_types(
    verb: &crate::ast::VerbDecl,
    layouts: &dyn crate::codegen::BackendLayoutProvider,
) -> std::collections::BTreeMap<BackendValueId, TypeName> {
    metadata::parameter_nominal_types(verb, layouts)
}

pub(super) fn lower_array_element(
    provider: &mut SharedExpressionProvider<'_>,
    collection: &Expr,
    index: &Expr,
    span: crate::lexer::SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    arrays::lower_array_element(provider, collection, index, span, instructions)
}

pub(super) fn lower_array_element_assignment(
    provider: &mut SharedExpressionProvider<'_>,
    collection: &Expr,
    index: &Expr,
    value: &Expr,
    span: crate::lexer::SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(), BackendContractError> {
    arrays::lower_array_element_assignment(provider, collection, index, value, span, instructions)
}

pub(super) fn lower_array_element_location(
    provider: &mut SharedExpressionProvider<'_>,
    collection: &Expr,
    index: &Expr,
    span: crate::lexer::SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<
    (
        BackendValueId,
        BackendValueId,
        crate::codegen::BackendLayout,
        crate::codegen::BackendLayout,
        TypeName,
        crate::codegen::BackendValueType,
        u32,
        bool,
    ),
    BackendContractError,
> {
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

pub(super) fn lower_field_assignment(
    provider: &mut SharedExpressionProvider<'_>,
    object: &Expr,
    field: &str,
    value: &Expr,
    span: crate::lexer::SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(), BackendContractError> {
    fields::lower_field_assignment(provider, object, field, value, span, instructions)
}

pub(super) fn lower_field_compound_assignment(
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

pub(super) fn lower_aggregate_expression(
    provider: &mut SharedExpressionProvider<'_>,
    expression: &Expr,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    match expression {
        Expr::StructLit { .. } => {
            structs::lower_struct_literal_expression(provider, expression, instructions)
        }
        Expr::FieldAccess { object, field, span } => {
            if let Expr::Identifier { name, .. } = object.as_ref() {
                let type_name = parse_type_name(name, *span);
                if provider.enum_variant(&type_name, field).is_some() {
                    return super::super::constructors::lower_named_unit_enum(
                        provider,
                        &type_name,
                        field,
                        &[],
                        *span,
                        instructions,
                    );
                }
            }
            fields::lower_field_access_expression(provider, object, field, *span, instructions)
        }
        Expr::Index { target, index, span } => {
            arrays::lower_array_element(provider, target, index, *span, instructions)
        }
        _ => Err(unsupported(expression_span(expression), "aggregate expression expected")),
    }
}

fn parse_type_name(name: &str, span: crate::lexer::SourceSpan) -> TypeName {
    crate::semantic::parse_type_name_key(name, span).unwrap_or(TypeName {
        name: name.to_owned(),
        arguments: Vec::new(),
        reference_role: None,
        span,
    })
}
