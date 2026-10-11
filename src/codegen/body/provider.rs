use super::super::body_contract::unsupported;
use super::{
    BackendBlock, BackendContractError, BackendFailure, BackendInstruction, BackendIntegerBinaryOp,
    BackendValueId, SymbolIdentity,
};
use crate::ast::{CompoundAssignmentOp, Expr};
use crate::lexer::SourceSpan;
use crate::semantic::LoopExitKind;

pub trait BackendBodyProvider {
    fn value_for(&self, expression: &Expr) -> Result<BackendValueId, BackendContractError>;
    fn failure_for(
        &self,
        _expression: &Expr,
    ) -> Result<Option<BackendFailure>, BackendContractError> {
        Ok(None)
    }

    fn callee_for(
        &self,
        callee: &str,
        span: SourceSpan,
    ) -> Result<SymbolIdentity, BackendContractError>;
}

pub trait BackendExpressionProvider {
    fn enter_scope(&mut self, _span: SourceSpan) {}

    fn exit_scope(
        &mut self,
        _span: SourceSpan,
        _instructions: &mut Vec<BackendInstruction>,
    ) -> Result<(), BackendContractError> {
        Ok(())
    }

    fn distribute_scope_cleanup(
        &mut self,
        _span: SourceSpan,
        _blocks: &mut Vec<BackendBlock>,
        _start: usize,
    ) -> Result<(), BackendContractError> {
        Ok(())
    }

    fn is_loop_exit(&self, _span: SourceSpan) -> bool {
        false
    }

    fn emit_loop_cleanup(
        &mut self,
        _span: SourceSpan,
        _kind: LoopExitKind,
        _instructions: &mut Vec<BackendInstruction>,
    ) -> Result<(), BackendContractError> {
        Ok(())
    }

    fn drop_binding(
        &mut self,
        _name: &str,
        span: SourceSpan,
        _instructions: &mut Vec<BackendInstruction>,
    ) -> Result<(), BackendContractError> {
        Err(unsupported(span, "explicit drop"))
    }

    fn bind_owner(
        &mut self,
        _name: &str,
        _value: BackendValueId,
        span: SourceSpan,
    ) -> Result<(), BackendContractError> {
        Err(unsupported(span, "owner declaration"))
    }

    fn assign_owner(
        &mut self,
        _name: &str,
        _value: BackendValueId,
        span: SourceSpan,
    ) -> Result<(), BackendContractError> {
        Err(unsupported(span, "assignment"))
    }

    fn assign_field(
        &mut self,
        _object: &Expr,
        _field: &str,
        _value: &Expr,
        span: SourceSpan,
        _instructions: &mut Vec<BackendInstruction>,
    ) -> Result<(), BackendContractError> {
        Err(unsupported(span, "aggregate field assignment"))
    }

    fn compound_assign_field(
        &mut self,
        _object: &Expr,
        _field: &str,
        _operator: CompoundAssignmentOp,
        _value: &Expr,
        span: SourceSpan,
        _instructions: &mut Vec<BackendInstruction>,
    ) -> Result<(), BackendContractError> {
        Err(unsupported(span, "aggregate field compound assignment"))
    }

    fn lower_integer_binary(
        &mut self,
        _operation: BackendIntegerBinaryOp,
        _left: BackendValueId,
        _right: BackendValueId,
        span: SourceSpan,
        _instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
        Err(unsupported(span, "integer binary operation"))
    }

    fn lower_expression(
        &mut self,
        expression: &Expr,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError>;

    fn lower_enum_variant_condition_value(
        &mut self,
        _subject: BackendValueId,
        _variant: &str,
        span: SourceSpan,
        _instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
        Err(unsupported(span, "enum case matching"))
    }

    fn bind_enum_variant_payload(
        &mut self,
        _subject: BackendValueId,
        _variant: &str,
        _payload: Option<&crate::ast::VariantPayload>,
        _mode: crate::ast::CaseMode,
        span: SourceSpan,
        _instructions: &mut Vec<BackendInstruction>,
    ) -> Result<(), BackendContractError> {
        Err(unsupported(span, "enum case payload binding"))
    }

    fn failure_for(
        &mut self,
        _expression: &Expr,
    ) -> Result<Option<BackendFailure>, BackendContractError> {
        Ok(None)
    }

    fn callee_for(
        &mut self,
        callee: &str,
        span: SourceSpan,
    ) -> Result<SymbolIdentity, BackendContractError>;
}
