use crate::ast::Argument;
use crate::codegen::{
    BackendContractError, BackendExpressionProvider, BackendInstruction, BackendPassingMode,
    BackendValueId, BackendValueType,
};
use crate::lexer::SourceSpan;

use super::super::super::SharedExpressionProvider;
use super::loan::{self, ScalarLoanWriteback};

pub(super) struct LoweredArguments {
    pub(super) values: Vec<BackendValueId>,
    pub(super) writebacks: Vec<ScalarLoanWriteback>,
}

pub(super) fn lower_arguments(
    provider: &mut SharedExpressionProvider<'_>,
    callee: &str,
    arguments: &[Argument],
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<LoweredArguments, BackendContractError> {
    let modes = provider.parameter_modes.get(callee).cloned().unwrap_or_default();
    let parameter_types = provider.parameter_types.get(callee).cloned().unwrap_or_default();
    let mut values = Vec::with_capacity(arguments.len());
    let mut writebacks = Vec::new();
    for (index, argument) in arguments.iter().enumerate() {
        let (value, writeback) =
            lower_argument(provider, argument, callee, modes.get(index), span, instructions)?;
        values.push(coerce_call_argument(
            provider,
            value,
            parameter_types.get(index).copied(),
            span,
            instructions,
        ));
        if let Some(writeback) = writeback {
            writebacks.push(writeback);
        }
    }
    Ok(LoweredArguments { values, writebacks })
}

fn lower_argument(
    provider: &mut SharedExpressionProvider<'_>,
    argument: &Argument,
    callee: &str,
    mode: Option<&BackendPassingMode>,
    span: SourceSpan,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(BackendValueId, Option<ScalarLoanWriteback>), BackendContractError> {
    if let Some(BackendPassingMode::ExclusiveLoan { layout }) = mode {
        let value = loan::lower_aggregate_loan_argument(
            provider,
            &argument.expression,
            callee,
            *layout,
            span,
            instructions,
        )?;
        return Ok((value, None));
    }
    let Some(BackendPassingMode::ExclusiveScalarLoan { ty }) = mode else {
        return Ok((provider.lower_expression(&argument.expression, instructions)?, None));
    };
    loan::lower_scalar_loan_argument(provider, argument, *ty, span, instructions)
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
