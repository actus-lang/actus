use super::super::super::{
    BackendContractError, BackendInstruction, BackendValueId, BackendValueType,
};
use super::super::support::{expression_span, parse_integer, unsupported};
use super::SharedExpressionProvider;
use crate::ast::Expr;
use crate::codegen::body::BackendExpressionProvider;
use crate::lexer::SourceSpan;

pub(super) fn lower_expression(
    provider: &mut SharedExpressionProvider<'_>,
    expression: &Expr,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    match expression {
        Expr::Identifier { name, span } => provider
            .binding_value(name)
            .ok_or_else(|| unsupported(*span, format!("unknown scalar binding `{name}`"))),
        Expr::Integer { value, suffix, span } => {
            let value = parse_integer(value).ok_or_else(|| {
                unsupported(*span, format!("invalid scalar integer literal `{value}`"))
            })?;
            let ty = super::literals::integer_literal_type(suffix.as_deref());
            let result = provider.allocate();
            instructions.push(BackendInstruction::Constant { result, ty, value, span: *span });
            provider.remember_value(result, ty);
            Ok(result)
        }
        Expr::BoolLiteral { value, span } => {
            lower_bool_literal(provider, *value, *span, instructions)
        }
        Expr::StringLiteral { value, span } => lower_string(provider, value, *span, instructions),
        Expr::BufferLiteral { length, span } => {
            super::literals::lower_buffer_literal(provider, length, *span, instructions)
        }
        Expr::Grouping { expression, .. } => lower_expression(provider, expression, instructions),
        Expr::Borrow { expression, span } => {
            lower_borrow(provider, expression, *span, instructions)
        }
        Expr::Cast { expression, target, span } => {
            if matches!(expression.as_ref(), Expr::Integer { .. }) {
                provider.lower_constant_cast(expression, target, *span, instructions)
            } else {
                provider.lower_runtime_cast(expression, target, *span, instructions)
            }
        }
        Expr::Try { expression, span } => {
            lower_same_result_try(provider, expression, *span, instructions)
        }
        Expr::Unary { .. } => provider.lower_unary(expression, instructions),
        Expr::Binary { .. } => provider.lower_binary(expression, instructions),
        Expr::Call { .. } => super::calls::lower_call(provider, expression, instructions),
        Expr::MethodCall { .. } => provider.lower_method_call(expression, instructions),
        Expr::StructLit { .. } | Expr::FieldAccess { .. } | Expr::Index { .. } => {
            super::aggregate::lower_aggregate_expression(provider, expression, instructions)
        }
        _ => Err(unsupported(
            expression_span(expression),
            "expression is outside the scalar shared lowering contract",
        )),
    }
}

fn lower_string(
    provider: &mut SharedExpressionProvider<'_>,
    value: &str,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let result = provider.allocate();
    provider.remember_value(result, BackendValueType::Pointer);
    if let Some(type_name) = crate::semantic::parse_type_name_key("String", span) {
        provider.remember_nominal_type(result, type_name);
    }
    instructions.push(BackendInstruction::StringLiteral { result, value: value.to_owned(), span });
    Ok(result)
}

fn lower_bool_literal(
    provider: &mut SharedExpressionProvider<'_>,
    value: bool,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let result = provider.allocate();
    instructions.push(BackendInstruction::Constant {
        result,
        ty: BackendValueType::Bool,
        value: u64::from(value),
        span,
    });
    provider.remember_value(result, BackendValueType::Bool);
    Ok(result)
}

fn lower_borrow(
    provider: &mut SharedExpressionProvider<'_>,
    expression: &Expr,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let value = lower_expression(provider, expression, instructions)?;
    if matches!(
        provider.value_type(value),
        Some(
            BackendValueType::Bool
                | BackendValueType::Integer { .. }
                | BackendValueType::Pointer
                | BackendValueType::NichePointerOption { .. },
        )
    ) {
        Ok(value)
    } else {
        Err(unsupported(span, "borrow requires a pointer-valued expression"))
    }
}

fn lower_same_result_try(
    provider: &mut SharedExpressionProvider<'_>,
    expression: &Expr,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let value = provider.lower_expression(expression, instructions)?;
    let return_type = provider
        .return_types
        .get(&provider.verb_name)
        .copied()
        .flatten()
        .ok_or_else(|| unsupported(span, "try propagation requires a Result return type"))?;
    if provider.value_type(value) != Some(return_type) {
        return Err(unsupported(span, "try propagation requires the same Result type"));
    }
    Ok(value)
}
