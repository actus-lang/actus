mod arguments;
mod constructors;
mod loan;
mod print;
mod resolution;
mod runtime;

use super::super::super::{
    BackendContractError, BackendExpressionProvider, BackendInstruction, BackendValueId,
};
use super::super::support::expression_span;
use super::SharedExpressionProvider;
use crate::ast::{Expr, IntrinsicKind, lookup_call_intrinsic};
use crate::lexer::SourceSpan;

pub(super) fn callee_for(
    provider: &SharedExpressionProvider<'_>,
    callee: &str,
    span: SourceSpan,
) -> Result<crate::codegen::SymbolIdentity, BackendContractError> {
    resolution::callee_for(provider, callee, span)
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
        return lower_copy_call(provider, arguments, *span, instructions);
    }
    if arguments.is_empty() && callee.starts_with("Array[") {
        return constructors::lower_array_constructor(provider, callee, *span, instructions);
    }
    if arguments.is_empty() && callee.starts_with("Arena[") {
        return super::arena::lower_constructor(provider, callee, *span, instructions);
    }
    let lowered = arguments::lower_arguments(provider, callee, arguments, *span, instructions)?;
    let arguments = lowered.values;
    if let Some((symbol, result_type)) = runtime::runtime_intrinsic(callee) {
        return runtime::lower_runtime_call(
            provider,
            callee,
            symbol,
            result_type,
            arguments,
            *span,
            instructions,
        );
    }
    if is_print_intrinsic_call(provider, callee, expression_arguments(expression)) {
        return print::lower_print_call(
            provider,
            callee,
            expression_arguments(expression),
            arguments,
            *span,
            instructions,
        );
    }
    lower_user_call(provider, callee, arguments, lowered.writebacks, *span, instructions)
}

fn is_print_intrinsic_call(
    provider: &SharedExpressionProvider<'_>,
    callee: &str,
    arguments: &[crate::ast::Argument],
) -> bool {
    if !matches!(lookup_call_intrinsic(callee), Some(IntrinsicKind::Print | IntrinsicKind::Eprint))
    {
        return false;
    }
    !(callee == "eprint"
        && arguments.len() == 1
        && arguments[0].name.as_deref() == Some("buffer")
        && provider.return_types.contains_key(callee))
}

fn lower_user_call(
    provider: &mut SharedExpressionProvider<'_>,
    callee: &str,
    arguments: Vec<BackendValueId>,
    writebacks: Vec<loan::ScalarLoanWriteback>,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let result_type = provider.return_types.get(callee).copied().flatten();
    let result = result_type.map(|_| provider.allocate());
    if let (Some(result), Some(result_type)) = (result, result_type) {
        provider.remember_value(result, result_type);
        remember_return_metadata(provider, result, callee);
    }
    instructions.push(BackendInstruction::Call {
        callee: provider.callee_for(callee, span)?,
        arguments,
        result,
        span,
    });
    loan::append_scalar_loan_writebacks(provider, writebacks, span, instructions);
    result
        .ok_or_else(|| super::super::support::unsupported(span, "unit call cannot produce a value"))
}

fn expression_arguments(expression: &Expr) -> &[crate::ast::Argument] {
    match expression {
        Expr::Call { arguments, .. } => arguments,
        _ => &[],
    }
}

fn is_scalar_copy_intrinsic(provider: &SharedExpressionProvider<'_>, callee: &str) -> bool {
    (callee == "copy" || callee.starts_with("copy__"))
        && !provider.return_types.contains_key(callee)
}

fn lower_copy_call(
    provider: &mut SharedExpressionProvider<'_>,
    arguments: &[crate::ast::Argument],
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<BackendValueId, BackendContractError> {
    let [argument] = arguments else {
        return Err(super::super::support::unsupported(span, "copy requires one value"));
    };
    provider.lower_expression(&argument.expression, instructions)
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
    let lowered = arguments::lower_arguments(provider, callee, arguments, *span, instructions)?;
    instructions.push(BackendInstruction::Call {
        callee: provider.callee_for(callee, *span)?,
        arguments: lowered.values,
        result: None,
        span: *span,
    });
    loan::append_scalar_loan_writebacks(provider, lowered.writebacks, *span, instructions);
    Ok(())
}
