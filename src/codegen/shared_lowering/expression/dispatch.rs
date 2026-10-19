use super::super::super::{
    BackendContractError, BackendInstruction, BackendValueId, BackendValueType,
};
use super::super::support::{
    expression_kind, expression_span, parse_integer, unsupported, unsupported_operation,
};
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
            lower_integer_literal(provider, value, suffix.as_deref(), *span, instructions)
        }
        Expr::FloatLiteral { value, suffix, span } => super::literals::lower_float_literal(
            provider,
            value,
            suffix.as_deref(),
            *span,
            instructions,
        ),
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
        Expr::If { condition, .. } if matches!(condition.as_ref(), Expr::BoolLiteral { .. }) => {
            lower_constant_if(provider, expression, instructions)
        }
        Expr::If { .. } => lower_scalar_if(provider, expression, instructions),
        Expr::Call { .. } => super::calls::lower_call(provider, expression, instructions),
        Expr::MethodCall { .. } => provider.lower_method_call(expression, instructions),
        Expr::StructLit { .. } | Expr::FieldAccess { .. } | Expr::Index { .. } => {
            super::aggregate::lower_aggregate_expression(provider, expression, instructions)
        }
        _ => Err(unsupported_operation(
            expression_span(expression),
            expression_kind(expression),
            "expression is outside the scalar shared lowering contract",
        )),
    }
}

fn lower_scalar_if(
    provider: &mut SharedExpressionProvider<'_>,
    expression: &Expr,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let Expr::If { condition, then_branch, else_branch, span } = expression else {
        unreachable!("scalar-if lowering received a non-if expression")
    };
    let Some(crate::ast::Stmt::Expression { expression: then_expression, .. }) =
        then_branch.statements.last()
    else {
        return Err(unsupported(*span, "scalar if branch requires a tail expression"));
    };
    let Some(crate::ast::IfBranch::Block(else_block)) = else_branch else {
        return Err(unsupported(*span, "scalar if requires an else block"));
    };
    let Some(crate::ast::Stmt::Expression { expression: else_expression, .. }) =
        else_block.statements.last()
    else {
        return Err(unsupported(*span, "scalar if branch requires a tail expression"));
    };
    if then_branch.statements.len() != 1 || else_block.statements.len() != 1 {
        return Err(unsupported(*span, "scalar if branches must be pure tail expressions"));
    }
    let condition = provider.lower_expression(condition, instructions)?;
    if provider.value_type(condition) != Some(BackendValueType::Bool) {
        return Err(unsupported(*span, "scalar if condition must be Bool"));
    }
    let then_value = provider.lower_expression(then_expression, instructions)?;
    let else_value = provider.lower_expression(else_expression, instructions)?;
    let Some(result_type) = provider.value_type(then_value) else {
        return Err(unsupported(*span, "scalar if then value type is unavailable"));
    };
    if provider.value_type(else_value) != Some(result_type)
        || !matches!(result_type, BackendValueType::Bool | BackendValueType::Integer { .. })
    {
        return Err(unsupported(*span, "scalar if branches must have the same scalar type"));
    }
    let result = provider.allocate();
    provider.remember_value(result, result_type);
    instructions.push(BackendInstruction::ScalarSelect {
        result,
        condition,
        then_value,
        else_value,
        ty: result_type,
        span: *span,
    });
    Ok(result)
}

fn lower_integer_literal(
    provider: &mut SharedExpressionProvider<'_>,
    value: &str,
    suffix: Option<&str>,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let value = parse_integer(value)
        .ok_or_else(|| unsupported(span, format!("invalid scalar integer literal `{value}`")))?;
    let ty = super::literals::integer_literal_type(suffix);
    let result = provider.allocate();
    instructions.push(BackendInstruction::Constant { result, ty, value, span });
    provider.remember_value(result, ty);
    Ok(result)
}

fn lower_constant_if(
    provider: &mut SharedExpressionProvider<'_>,
    expression: &Expr,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let Expr::If { condition, then_branch, else_branch, span } = expression else {
        unreachable!("constant-if lowering received a non-if expression")
    };
    let selected = match condition.as_ref() {
        Expr::BoolLiteral { value: true, .. } => Some(then_branch),
        Expr::BoolLiteral { value: false, .. } => match else_branch.as_ref() {
            Some(crate::ast::IfBranch::Block(block)) => Some(block),
            Some(crate::ast::IfBranch::ElseIf(nested)) => {
                return lower_expression(provider, nested.as_ref(), instructions);
            }
            None => None,
        },
        _ => None,
    };
    let Some(crate::ast::Block { statements, .. }) = selected else {
        return Err(unsupported(*span, "constant if expression has no selected branch"));
    };
    let Some(crate::ast::Stmt::Expression { expression, .. }) = statements.last() else {
        return Err(unsupported(*span, "constant if branch requires a tail expression"));
    };
    lower_expression(provider, expression, instructions)
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
                | BackendValueType::Aggregate { .. }
                | BackendValueType::Enum { .. }
                | BackendValueType::Region { .. }
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
