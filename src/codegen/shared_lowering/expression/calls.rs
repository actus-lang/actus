use super::super::super::{
    BackendContractError, BackendExpressionProvider, BackendInstruction, BackendValueId,
    BackendValueType,
};
use super::super::support::expression_span;
use super::SharedExpressionProvider;
use super::aggregate::lower_array_element_location;
use crate::ast::{Expr, IntrinsicKind, lookup_call_intrinsic};
use crate::codegen::BackendPassingMode;
use crate::codegen::{SymbolIdentity, SymbolKind};
use crate::lexer::SourceSpan;

pub(super) fn callee_for(
    provider: &SharedExpressionProvider<'_>,
    callee: &str,
    span: SourceSpan,
) -> Result<SymbolIdentity, BackendContractError> {
    provider
        .external_symbols
        .get(callee)
        .cloned()
        .or_else(|| {
            provider
                .bindings
                .map(|bindings| bindings.external_symbol(provider.namespace, callee))
                .filter(|symbol| *symbol != callee)
                .and_then(|symbol| SymbolIdentity::external_raw(symbol).ok())
        })
        .or_else(|| {
            provider
                .return_types
                .contains_key(callee)
                .then(|| SymbolIdentity::new(provider.namespace, SymbolKind::Verb, callee))
                .and_then(Result::ok)
        })
        .map_or_else(|| SymbolIdentity::new(provider.namespace, SymbolKind::Verb, callee), Ok)
        .map_err(|error| super::super::support::unsupported(span, error.to_string()))
}

pub(super) fn lower_call(
    provider: &mut SharedExpressionProvider<'_>,
    expression: &Expr,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let Expr::Call { callee, arguments, span } = expression else {
        return Err(super::super::support::unsupported(
            expression_span(expression),
            "call expression expected",
        ));
    };
    if let Some(value) = super::layout::lower_intrinsic(provider, callee, *span, instructions)? {
        return Ok(value);
    }
    if is_scalar_copy_intrinsic(provider, callee) {
        let [argument] = arguments.as_slice() else {
            return Err(super::super::support::unsupported(*span, "copy requires one value"));
        };
        return provider.lower_expression(&argument.expression, instructions);
    }
    if arguments.is_empty() && callee.starts_with("Array[") {
        return lower_array_constructor(provider, callee, *span, instructions);
    }
    if arguments.is_empty() && callee.starts_with("Arena[") {
        return super::arena::lower_constructor(provider, callee, *span, instructions);
    }
    let lowered = lower_arguments(provider, callee, arguments, *span, instructions)?;
    let arguments = lowered.values;
    if let Some((symbol, result_type)) = runtime_intrinsic(callee) {
        return lower_runtime_call(
            provider,
            callee,
            symbol,
            result_type,
            arguments,
            *span,
            instructions,
        );
    }
    if lookup_call_intrinsic(callee) == Some(IntrinsicKind::Print) {
        return lower_print_call(provider, arguments, *span, instructions);
    }
    let result_type = provider.return_types.get(callee).copied().flatten();
    let result = result_type.map(|_| provider.allocate());
    if let (Some(result), Some(result_type)) = (result, result_type) {
        provider.remember_value(result, result_type);
        remember_return_metadata(provider, result, callee);
    }
    instructions.push(BackendInstruction::Call {
        callee: provider.callee_for(callee, *span)?,
        arguments,
        result,
        span: *span,
    });
    append_scalar_loan_writebacks(provider, lowered.writebacks, *span, instructions);
    result.ok_or_else(|| {
        super::super::support::unsupported(*span, "unit call cannot produce a value")
    })
}

fn is_scalar_copy_intrinsic(provider: &SharedExpressionProvider<'_>, callee: &str) -> bool {
    (callee == "copy" || callee.starts_with("copy__"))
        && !provider.return_types.contains_key(callee)
}

fn remember_return_metadata(
    provider: &mut SharedExpressionProvider<'_>,
    result: BackendValueId,
    callee: &str,
) {
    if let Some(Some(nominal_type)) = provider.return_nominal_types.get(callee) {
        provider.remember_nominal_type(result, nominal_type.clone());
    }
    if let Some(payload_types) = provider.return_payload_types.get(callee) {
        for (discriminant, payload_type) in payload_types.iter().enumerate() {
            if let Some(payload_type) = payload_type {
                provider.remember_enum_payload(result, discriminant as u32, *payload_type);
            }
        }
    }
}

pub(super) fn lower_unit_call(
    provider: &mut SharedExpressionProvider<'_>,
    expression: &Expr,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(), BackendContractError> {
    let Expr::Call { callee, arguments, span } = expression else {
        return Err(super::super::support::unsupported(
            expression_span(expression),
            "unit call expected",
        ));
    };
    let lowered = lower_arguments(provider, callee, arguments, *span, instructions)?;
    let arguments = lowered.values;
    instructions.push(BackendInstruction::Call {
        callee: provider.callee_for(callee, *span)?,
        arguments,
        result: None,
        span: *span,
    });
    append_scalar_loan_writebacks(provider, lowered.writebacks, *span, instructions);
    Ok(())
}

struct LoweredArguments {
    values: Vec<BackendValueId>,
    writebacks: Vec<ScalarLoanWriteback>,
}

struct ScalarLoanWriteback {
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

fn lower_arguments(
    provider: &mut SharedExpressionProvider<'_>,
    callee: &str,
    arguments: &[crate::ast::Argument],
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<LoweredArguments, BackendContractError> {
    let modes = provider.parameter_modes.get(callee).cloned().unwrap_or_default();
    let parameter_types = provider.parameter_types.get(callee).cloned().unwrap_or_default();
    let mut values = Vec::with_capacity(arguments.len());
    let mut writebacks = Vec::new();
    for (index, argument) in arguments.iter().enumerate() {
        if let Some(BackendPassingMode::ExclusiveLoan { layout }) = modes.get(index) {
            let value = lower_aggregate_loan_argument(
                provider,
                &argument.expression,
                callee,
                *layout,
                span,
                instructions,
            )?;
            values.push(coerce_call_argument(
                provider,
                value,
                parameter_types.get(index).copied(),
                span,
                instructions,
            ));
            continue;
        }
        let Some(BackendPassingMode::ExclusiveScalarLoan { ty }) = modes.get(index) else {
            let value = provider.lower_expression(&argument.expression, instructions)?;
            values.push(coerce_call_argument(
                provider,
                value,
                parameter_types.get(index).copied(),
                span,
                instructions,
            ));
            continue;
        };
        let (value, name, place, field) = match &argument.expression {
            crate::ast::Expr::Identifier { name, .. } => (
                provider.lower_expression(&argument.expression, instructions)?,
                Some(name.clone()),
                None,
                None,
            ),
            crate::ast::Expr::Index { target, index, .. } => {
                let (
                    base,
                    index,
                    aggregate_layout,
                    element_layout,
                    _,
                    element_type,
                    capacity,
                    big_endian,
                ) = lower_array_element_location(provider, target, index, span, instructions)?;
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
                (
                    value,
                    None,
                    Some(ArrayLoanPlace {
                        base,
                        index,
                        aggregate_layout,
                        element_layout,
                        element_type,
                        capacity,
                    }),
                    None,
                )
            }
            crate::ast::Expr::FieldAccess { object, field, .. } => {
                let base = provider.lower_expression(object, instructions)?;
                let type_name = provider.nominal_type(base).cloned().ok_or_else(|| {
                    super::super::support::unsupported(
                        span,
                        "ins field place has no aggregate type",
                    )
                })?;
                let aggregate_layout =
                    provider.layouts.layout_for(&type_name).ok_or_else(|| {
                        super::super::support::unsupported(
                            span,
                            "ins field place has no aggregate layout",
                        )
                    })?;
                let metadata = provider.layouts.field_for(&type_name, field).ok_or_else(|| {
                    super::super::support::unsupported(span, "ins field place has no field layout")
                })?;
                let ty = super::super::support::backend_value_type(provider.layouts, &metadata.ty)
                    .ok_or_else(|| {
                        super::super::support::unsupported(span, "ins field has no scalar type")
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
                (
                    value,
                    None,
                    None,
                    Some(AggregateFieldLoanPlace {
                        base,
                        aggregate_layout,
                        offset: metadata.offset,
                        field_layout: metadata.layout,
                        ty,
                    }),
                )
            }
            _ => {
                return Err(super::super::support::unsupported(
                    span,
                    "scalar `ins` call arguments require an addressable place",
                ));
            }
        };
        let address = provider.allocate();
        instructions.push(BackendInstruction::ScalarLoanMaterialize {
            result: address,
            value,
            ty: *ty,
            span,
        });
        values.push(address);
        writebacks.push(ScalarLoanWriteback { name, address, ty: *ty, place, field });
    }
    Ok(LoweredArguments { values, writebacks })
}

fn coerce_call_argument(
    provider: &mut SharedExpressionProvider<'_>,
    value: BackendValueId,
    expected: Option<BackendValueType>,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> BackendValueId {
    let Some(BackendValueType::Integer { .. }) = expected else { return value };
    let Some(source_type @ BackendValueType::Integer { .. }) = provider.value_type(value) else {
        return value;
    };
    if Some(source_type) == expected {
        return value;
    }
    let converted = provider.allocate();
    provider.remember_value(converted, expected.expect("integer expected"));
    instructions.push(BackendInstruction::IntegerCast {
        result: converted,
        source: value,
        source_type,
        target_type: expected.expect("integer expected"),
        span,
    });
    converted
}

fn lower_aggregate_loan_argument(
    provider: &mut SharedExpressionProvider<'_>,
    expression: &crate::ast::Expr,
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
        return Err(super::super::support::unsupported(
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

fn append_scalar_loan_writebacks(
    provider: &mut SharedExpressionProvider<'_>,
    writebacks: Vec<ScalarLoanWriteback>,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) {
    for writeback in writebacks {
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
            super::state::set_binding_value(provider, &name, value);
        }
    }
}

fn lower_runtime_call(
    provider: &mut SharedExpressionProvider<'_>,
    callee: &str,
    symbol: &str,
    result_type: crate::codegen::BackendValueType,
    arguments: Vec<BackendValueId>,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let arguments = normalize_runtime_arguments(provider, callee, arguments, span, instructions)?;
    let result = provider.allocate();
    provider.remember_value(result, result_type);
    instructions.push(BackendInstruction::Call {
        callee: SymbolIdentity::external_raw(symbol)
            .map_err(|error| super::super::support::unsupported(span, error.to_string()))?,
        arguments,
        result: Some(result),
        span,
    });
    Ok(result)
}

fn normalize_runtime_arguments(
    provider: &mut SharedExpressionProvider<'_>,
    callee: &str,
    mut arguments: Vec<BackendValueId>,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<Vec<BackendValueId>, BackendContractError> {
    if lookup_call_intrinsic(callee) != Some(IntrinsicKind::Append) {
        return Ok(arguments);
    }
    let byte = arguments
        .get(1)
        .copied()
        .ok_or_else(|| super::super::support::unsupported(span, "append requires a byte"))?;
    let source_type = provider.value_type(byte).ok_or_else(|| {
        super::super::support::unsupported(span, "append byte type is unavailable")
    })?;
    let target_type = crate::codegen::BackendValueType::Integer { signed: false, width: 8 };
    if source_type != target_type {
        let converted = provider.allocate();
        provider.remember_value(converted, target_type);
        instructions.push(BackendInstruction::IntegerCast {
            result: converted,
            source: byte,
            source_type,
            target_type,
            span,
        });
        arguments[1] = converted;
    }
    Ok(arguments)
}

fn runtime_intrinsic(callee: &str) -> Option<(&'static str, crate::codegen::BackendValueType)> {
    match lookup_call_intrinsic(callee) {
        Some(IntrinsicKind::Append) => Some((
            crate::runtime::BUFFER_APPEND_SYMBOL,
            crate::codegen::BackendValueType::Integer { signed: false, width: 8 },
        )),
        Some(IntrinsicKind::BufferLength) => Some((
            crate::runtime::BUFFER_LENGTH_SYMBOL,
            crate::codegen::BackendValueType::Integer { signed: true, width: 32 },
        )),
        Some(IntrinsicKind::Crc32) => Some((
            crate::runtime::BUFFER_CRC32_SYMBOL,
            crate::codegen::BackendValueType::Integer { signed: true, width: 64 },
        )),
        Some(IntrinsicKind::Crc32Matches) => Some((
            crate::runtime::BUFFER_CRC32_MATCHES_SYMBOL,
            crate::codegen::BackendValueType::Integer { signed: true, width: 64 },
        )),
        Some(IntrinsicKind::ValidateFixedFrame) => Some((
            crate::runtime::BUFFER_VALIDATE_FIXED_FRAME_SYMBOL,
            crate::codegen::BackendValueType::Integer { signed: true, width: 64 },
        )),
        _ => None,
    }
}

fn lower_print_call(
    provider: &mut SharedExpressionProvider<'_>,
    arguments: Vec<BackendValueId>,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let [value] = arguments.as_slice() else {
        return Err(super::super::support::unsupported(span, "print requires one value"));
    };
    let symbol = match provider.value_type(*value) {
        Some(crate::codegen::BackendValueType::Integer { .. }) => crate::runtime::PRINT_INT_SYMBOL,
        Some(crate::codegen::BackendValueType::Pointer)
            if provider.nominal_type(*value).is_some_and(|ty| ty.name == "Buffer") =>
        {
            crate::runtime::PRINT_BUFFER_STDOUT_SYMBOL
        }
        Some(crate::codegen::BackendValueType::Pointer)
            if provider.nominal_type(*value).is_some_and(|ty| ty.name == "String") =>
        {
            crate::runtime::WRITE_STRING_STDOUT_SYMBOL
        }
        Some(crate::codegen::BackendValueType::Pointer) => crate::runtime::PRINT_STRING_SYMBOL,
        _ => return Err(super::super::support::unsupported(span, "print value type")),
    };
    let callee = SymbolIdentity::external_raw(symbol)
        .map_err(|error| super::super::support::unsupported(span, error.to_string()))?;
    let result = provider.allocate();
    let ty = crate::codegen::BackendValueType::Integer { signed: true, width: 32 };
    provider.remember_value(result, ty);
    instructions.push(BackendInstruction::Call { callee, arguments, result: Some(result), span });
    Ok(result)
}

fn lower_array_constructor(
    provider: &mut SharedExpressionProvider<'_>,
    callee: &str,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let type_name = crate::semantic::parse_type_name_key(callee, span).ok_or_else(|| {
        super::super::support::unsupported(span, "malformed fixed-array constructor")
    })?;
    if type_name.name != "Array" || type_name.arguments.len() != 2 {
        return Err(super::super::support::unsupported(
            span,
            "fixed-array constructor requires Array[T, N]",
        ));
    }
    let capacity = type_name.arguments[1]
        .name
        .parse::<u32>()
        .map_err(|_| super::super::support::unsupported(span, "array capacity is not concrete"))?;
    if capacity == 0 {
        return Err(super::super::support::unsupported(
            span,
            "fixed-array capacity must be non-zero",
        ));
    }
    let layout = provider
        .layouts
        .layout_for(&type_name)
        .ok_or_else(|| super::super::support::unsupported(span, "array has no neutral layout"))?;
    let element_layout = provider.layouts.layout_for(&type_name.arguments[0]).ok_or_else(|| {
        super::super::support::unsupported(span, "array element has no neutral layout")
    })?;
    if u64::from(element_layout.size) * u64::from(capacity) != u64::from(layout.size) {
        return Err(super::super::support::unsupported(
            span,
            "array layout does not match capacity",
        ));
    }
    let result = provider.allocate();
    provider.remember_value(result, super::super::super::BackendValueType::Aggregate { layout });
    provider.remember_nominal_type(result, type_name);
    instructions.push(BackendInstruction::AggregateConstruct {
        result,
        layout,
        fields: Vec::new(),
        span,
    });
    Ok(result)
}
