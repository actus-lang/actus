use crate::ast::{Argument, Expr};
use crate::codegen::{
    BackendContractError, BackendExpressionProvider, BackendInstruction, BackendValueId,
    BackendValueType,
};
use crate::lexer::SourceSpan;

use super::super::SharedExpressionProvider;
use super::super::aggregate::lower_array_element_location;

pub(super) struct ScalarLoanWriteback {
    name: Option<String>,
    address: BackendValueId,
    ty: BackendValueType,
    place: Option<ArrayLoanPlace>,
    field: Option<AggregateFieldLoanPlace>,
}

struct AggregateFieldLoanPlace {
    base: BackendValueId,
    aggregate_layout: crate::codegen::BackendLayout,
    offset: u32,
    field_layout: crate::codegen::BackendLayout,
    ty: BackendValueType,
}

struct ArrayLoanPlace {
    base: BackendValueId,
    index: BackendValueId,
    aggregate_layout: crate::codegen::BackendLayout,
    element_layout: crate::codegen::BackendLayout,
    element_type: BackendValueType,
    capacity: u32,
}

pub(super) fn lower_aggregate_loan_argument(
    provider: &mut SharedExpressionProvider<'_>,
    expression: &Expr,
    callee: &str,
    layout: crate::codegen::BackendLayout,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let value = provider.lower_expression(expression, instructions)?;
    let caller_owned_storage = match provider.value_type(value) {
        Some(BackendValueType::Aggregate { layout: value_layout })
        | Some(BackendValueType::Region { layout: value_layout })
        | Some(BackendValueType::Enum {
            layout: crate::codegen::BackendEnumLayout { layout: value_layout, .. },
        }) => value_layout == layout,
        Some(BackendValueType::Pointer) => provider
            .nominal_type(value)
            .and_then(|type_name| provider.layouts.layout_for(type_name))
            .is_some_and(|value_layout| value_layout == layout),
        _ => false,
    };
    if !caller_owned_storage {
        return Err(super::super::super::support::unsupported(
            span,
            format!(
                "aggregate `ins` call argument for `{callee}` must resolve to caller-owned storage (value type: {:?}, nominal type: {:?}, expected layout: {:?})",
                provider.value_type(value),
                provider.nominal_type(value),
                layout
            ),
        ));
    }
    Ok(value)
}

pub(super) fn lower_scalar_loan_argument(
    provider: &mut SharedExpressionProvider<'_>,
    argument: &Argument,
    ty: BackendValueType,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(BackendValueId, Option<ScalarLoanWriteback>), BackendContractError> {
    let (value, writeback) = match &argument.expression {
        Expr::Identifier { name, .. } => (
            provider.lower_expression(&argument.expression, instructions)?,
            Some(ScalarLoanWriteback {
                name: Some(name.clone()),
                address: BackendValueId(0),
                ty,
                place: None,
                field: None,
            }),
        ),
        Expr::Index { target, index, .. } => {
            lower_array_loan(provider, target, index, ty, span, instructions)?
        }
        Expr::FieldAccess { object, field, .. } => {
            lower_field_loan(provider, object, field, ty, span, instructions)?
        }
        _ => {
            return Err(super::super::super::support::unsupported(
                span,
                "scalar `ins` call arguments require an addressable place",
            ));
        }
    };
    let address = provider.allocate();
    instructions.push(BackendInstruction::ScalarLoanMaterialize {
        result: address,
        value,
        ty,
        span,
    });
    let writeback = writeback.map(|mut writeback| {
        writeback.address = address;
        writeback
    });
    Ok((address, writeback))
}

fn lower_array_loan(
    provider: &mut SharedExpressionProvider<'_>,
    target: &Expr,
    index: &Expr,
    ty: BackendValueType,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(BackendValueId, Option<ScalarLoanWriteback>), BackendContractError> {
    let (base, index, aggregate_layout, element_layout, _, element_type, capacity, big_endian) =
        lower_array_element_location(provider, target, index, span, instructions)?;
    let value = provider.allocate();
    provider.remember_value(value, element_type);
    instructions.push(BackendInstruction::ArrayElementLoad {
        result: value,
        base,
        index,
        aggregate_layout,
        element_layout,
        element_type,
        capacity,
        big_endian,
        span,
    });
    Ok((
        value,
        Some(ScalarLoanWriteback {
            name: None,
            address: BackendValueId(0),
            ty,
            place: Some(ArrayLoanPlace {
                base,
                index,
                aggregate_layout,
                element_layout,
                element_type,
                capacity,
            }),
            field: None,
        }),
    ))
}

fn lower_field_loan(
    provider: &mut SharedExpressionProvider<'_>,
    object: &Expr,
    field: &str,
    ty: BackendValueType,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(BackendValueId, Option<ScalarLoanWriteback>), BackendContractError> {
    let base = provider.lower_expression(object, instructions)?;
    let type_name = provider.nominal_type(base).cloned().ok_or_else(|| {
        super::super::super::support::unsupported(span, "ins field place has no aggregate type")
    })?;
    let aggregate_layout = provider.layouts.layout_for(&type_name).ok_or_else(|| {
        super::super::super::support::unsupported(span, "ins field place has no aggregate layout")
    })?;
    let metadata = provider.layouts.field_for(&type_name, field).ok_or_else(|| {
        super::super::super::support::unsupported(span, "ins field place has no field layout")
    })?;
    let value = provider.allocate();
    provider.remember_value(value, ty);
    instructions.push(BackendInstruction::AggregateFieldLoad {
        result: value,
        base,
        aggregate_layout,
        offset: metadata.offset,
        field_layout: metadata.layout,
        ty,
        span,
    });
    Ok((
        value,
        Some(ScalarLoanWriteback {
            name: None,
            address: BackendValueId(0),
            ty,
            place: None,
            field: Some(AggregateFieldLoanPlace {
                base,
                aggregate_layout,
                offset: metadata.offset,
                field_layout: metadata.layout,
                ty,
            }),
        }),
    ))
}

pub(super) fn append_scalar_loan_writebacks(
    provider: &mut SharedExpressionProvider<'_>,
    writebacks: Vec<ScalarLoanWriteback>,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) {
    for writeback in writebacks {
        append_writeback(provider, writeback, span, instructions);
    }
}

fn append_writeback(
    provider: &mut SharedExpressionProvider<'_>,
    writeback: ScalarLoanWriteback,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) {
    let ScalarLoanWriteback { name, address, ty, place, field } = writeback;
    let value = provider.allocate();
    provider.remember_value(value, ty);
    instructions.push(BackendInstruction::ScalarLoanLoad { result: value, address, ty, span });
    if let Some(place) = place {
        instructions.push(BackendInstruction::ArrayElementStore {
            base: place.base,
            index: place.index,
            aggregate_layout: place.aggregate_layout,
            element_layout: place.element_layout,
            element_type: place.element_type,
            value,
            capacity: place.capacity,
            big_endian: false,
            span,
        });
    } else if let Some(field) = field {
        instructions.push(BackendInstruction::AggregateFieldStore {
            base: field.base,
            aggregate_layout: field.aggregate_layout,
            offset: field.offset,
            field_layout: field.field_layout,
            value,
            ty: field.ty,
            span,
        });
    } else if let Some(name) = name {
        super::super::state::set_binding_value(provider, &name, value);
    }
}
