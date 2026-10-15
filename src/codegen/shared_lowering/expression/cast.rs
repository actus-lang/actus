use crate::ast::Expr;
use crate::lexer::SourceSpan;

use super::super::support::{
    integer_value_fits, parse_integer, scalar_backend_type, scalar_integer_type, unsupported,
};
use super::SharedExpressionProvider;
use crate::codegen::{
    BackendContractError, BackendExpressionProvider, BackendInstruction, BackendValueId,
    BackendValueType,
};

impl SharedExpressionProvider<'_> {
    pub(super) fn lower_constant_cast(
        &mut self,
        expression: &Expr,
        target: &crate::ast::TypeName,
        span: SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
        let Expr::Integer { value, .. } = expression else {
            return Err(unsupported(span, "constant integer casts require an integer literal"));
        };
        let Some((signed, width)) = scalar_integer_type(target) else {
            return Err(unsupported(span, "cast target is not a supported scalar integer"));
        };
        let Some(value) = parse_integer(value) else {
            return Err(unsupported(span, "cast source is not a valid integer literal"));
        };
        if !integer_value_fits(value, signed, width) {
            return Err(unsupported(span, "constant integer cast overflows its target type"));
        }
        let result = self.allocate();
        let target_type = BackendValueType::Integer { signed, width };
        self.remember_value(result, target_type);
        instructions.push(BackendInstruction::Constant { result, ty: target_type, value, span });
        Ok(result)
    }

    pub(super) fn lower_runtime_cast(
        &mut self,
        expression: &Expr,
        target: &crate::ast::TypeName,
        span: SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
        let source = self.lower_expression(expression, instructions)?;
        let source_type = self
            .value_type(source)
            .ok_or_else(|| unsupported(span, "cast source type is unavailable"))?;
        let target_type = scalar_backend_type(target)
            .ok_or_else(|| unsupported(span, "cast target is not a supported scalar integer"))?;
        if !matches!(source_type, BackendValueType::Integer { .. }) {
            return Err(unsupported(span, "cast source is not a supported scalar integer"));
        }
        let result = self.allocate();
        self.remember_value(result, target_type);
        instructions.push(BackendInstruction::IntegerCast {
            result,
            source,
            source_type,
            target_type,
            span,
        });
        Ok(result)
    }
}
