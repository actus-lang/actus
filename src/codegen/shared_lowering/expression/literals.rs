use crate::ast::Expr;
use crate::codegen::body::BackendExpressionProvider;
use crate::lexer::SourceSpan;

use super::super::super::{
    BackendContractError, BackendInstruction, BackendValueId, BackendValueType, SymbolIdentity,
};
use super::super::support::unsupported;
use super::SharedExpressionProvider;

pub(super) fn integer_literal_type(suffix: Option<&str>) -> BackendValueType {
    let Some(suffix) = suffix else {
        return BackendValueType::Integer { signed: true, width: 32 };
    };
    let (signed, width) = match suffix {
        "u8" => (false, 8),
        "u16" => (false, 16),
        "u32" => (false, 32),
        "u64" | "usize" => (false, 64),
        "i8" => (true, 8),
        "i16" => (true, 16),
        "i32" => (true, 32),
        "i64" | "isize" => (true, 64),
        _ => (true, 32),
    };
    BackendValueType::Integer { signed, width }
}

pub(super) fn lower_float_literal(
    provider: &mut SharedExpressionProvider<'_>,
    value: &str,
    suffix: Option<&str>,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let ty = match suffix {
        Some("f32") => BackendValueType::Float { width: 32 },
        Some("f64") | None => BackendValueType::Float { width: 64 },
        Some(other) => return Err(unsupported(span, format!("invalid float suffix `{other}`"))),
    };
    let bits = match ty {
        BackendValueType::Float { width: 32 } => value
            .parse::<f32>()
            .map(|value| u64::from(value.to_bits()))
            .map_err(|_| unsupported(span, format!("invalid f32 literal `{value}`")))?,
        BackendValueType::Float { width: 64 } => value
            .parse::<f64>()
            .map(f64::to_bits)
            .map_err(|_| unsupported(span, format!("invalid f64 literal `{value}`")))?,
        _ => unreachable!(),
    };
    let result = provider.allocate();
    provider.remember_value(result, ty);
    instructions.push(BackendInstruction::FloatConstant { result, ty, bits, span });
    Ok(result)
}

pub(super) fn lower_buffer_literal(
    provider: &mut SharedExpressionProvider<'_>,
    expression: &Expr,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let length = provider.lower_expression(expression, instructions)?;
    let length_type = provider
        .value_type(length)
        .ok_or_else(|| unsupported(span, "Buffer length type is unavailable"))?;
    let length = if length_type == (BackendValueType::Integer { signed: false, width: 64 }) {
        length
    } else {
        let converted = provider.allocate();
        let target = BackendValueType::Integer { signed: false, width: 64 };
        instructions.push(BackendInstruction::IntegerCast {
            result: converted,
            source: length,
            source_type: length_type,
            target_type: target,
            span,
        });
        provider.remember_value(converted, target);
        converted
    };
    let result = provider.allocate();
    provider.remember_value(result, BackendValueType::Pointer);
    if let Some(type_name) = crate::semantic::parse_type_name_key("Buffer", span) {
        provider.remember_nominal_type(result, type_name);
    }
    instructions.push(BackendInstruction::Call {
        callee: SymbolIdentity::external_raw(crate::runtime::BUFFER_ALLOCATE_SYMBOL)
            .map_err(|error| unsupported(span, error.to_string()))?,
        arguments: vec![length],
        result: Some(result),
        span,
    });
    Ok(result)
}
