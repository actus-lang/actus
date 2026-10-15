use std::collections::BTreeMap;

use cranelift_codegen::ir::InstBuilder;
use cranelift_codegen::ir::condcodes::FloatCC;
use cranelift_frontend::FunctionBuilder;

use crate::codegen::{
    BackendEmitError, BackendInstruction, BackendIntegerCompareOp, BackendValueId, BackendValueType,
};

pub(super) fn lower_float_compare(
    builder: &mut FunctionBuilder<'_>,
    instruction: &BackendInstruction,
    values: &mut BTreeMap<BackendValueId, cranelift_codegen::ir::Value>,
) -> Result<(), BackendEmitError> {
    let BackendInstruction::FloatCompare { result, operation, left, right, ty, .. } = instruction
    else {
        return Err(BackendEmitError("Cranelift float comparison expected".to_owned()));
    };
    if !matches!(ty, BackendValueType::Float { width: 32 | 64 }) {
        return Err(BackendEmitError("Cranelift float comparison requires f32 or f64".to_owned()));
    }
    let left = values.get(left).copied().ok_or_else(|| {
        BackendEmitError(format!(
            "Cranelift float comparison left operand {} is unavailable",
            left.0
        ))
    })?;
    let right = values.get(right).copied().ok_or_else(|| {
        BackendEmitError(format!(
            "Cranelift float comparison right operand {} is unavailable",
            right.0
        ))
    })?;
    let condition = match operation {
        BackendIntegerCompareOp::Equals => FloatCC::Equal,
        BackendIntegerCompareOp::NotEquals => FloatCC::NotEqual,
        BackendIntegerCompareOp::LessThan => FloatCC::LessThan,
        BackendIntegerCompareOp::LessEquals => FloatCC::LessThanOrEqual,
        BackendIntegerCompareOp::GreaterThan => FloatCC::GreaterThan,
        BackendIntegerCompareOp::GreaterEquals => FloatCC::GreaterThanOrEqual,
    };
    values.insert(*result, builder.ins().fcmp(condition, left, right));
    Ok(())
}
