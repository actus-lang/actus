use super::super::super::{
    BackendBlockId, BackendCleanupOrigin, BackendContractError, BackendExpressionProvider,
    BackendInstruction, BackendIntegerBinaryOp, BackendIntegerUnaryOp, BackendValueId,
    BackendValueType, SymbolIdentity,
};
use super::super::errors::{expression_span, unsupported};
use super::SharedExpressionProvider;
use super::{aggregate, arena, array_ops, calls, case, cleanup, dispatch, drop, state};
use crate::ast::{BinaryOp, CaseMode, Expr, UnaryOp, VariantPayload};
use crate::lexer::SourceSpan;
use crate::semantic::LoopExitKind;
use std::collections::BTreeMap;

pub(in crate::codegen::shared_lowering) fn lower_unit_call(
    provider: &mut SharedExpressionProvider<'_>,
    expression: &Expr,
    instructions: &mut Vec<BackendInstruction>,
) -> Result<(), BackendContractError> {
    calls::lower_unit_call(provider, expression, instructions)
}

include!("provider.rs");

impl SharedExpressionProvider<'_> {
    pub(in crate::codegen::shared_lowering) fn return_type_for(
        &self,
        callee: &str,
    ) -> Option<Option<BackendValueType>> {
        self.return_types.get(callee).copied().or_else(|| {
            callee.rsplit_once("::").and_then(|(_, member)| self.return_types.get(member).copied())
        })
    }

    pub(in crate::codegen::shared_lowering) fn call_returns_unit(&self, callee: &str) -> bool {
        self.return_type_for(callee).is_some_and(|return_type| return_type.is_none())
    }

    pub(super) fn lower_method_call(
        &mut self,
        expression: &Expr,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
        if arena::is_place_call(self, expression) {
            return arena::lower_place(self, expression, instructions);
        }
        let Expr::MethodCall { receiver, method, arguments, span } = expression else {
            return Err(unsupported(expression_span(expression), "method call expected"));
        };
        let receiver_is_binding = self.is_binding_receiver(receiver);
        if !receiver_is_binding {
            return super::super::constructors::lower_method_call(self, expression, instructions);
        }
        let receiver_value = if let Some(address) = self.scalar_loan_parameter_address(receiver) {
            address
        } else {
            dispatch::lower_expression(self, receiver, instructions)?
        };
        let receiver_type = self
            .nominal_type(receiver_value)
            .ok_or_else(|| unsupported(*span, "performance receiver has no nominal type"))?;
        let key = (receiver_type.canonical_key(), method.clone());
        let callee = self.method_callee(&key, method, *span)?;
        let mut lowered_arguments = Vec::with_capacity(arguments.len() + 1);
        lowered_arguments.push(receiver_value);
        lowered_arguments.extend(
            arguments
                .iter()
                .map(|argument| {
                    dispatch::lower_expression(self, &argument.expression, instructions)
                })
                .collect::<Result<Vec<_>, _>>()?,
        );
        let result_type = self.method_result_type(&key, method);
        let result = result_type.map(|_| self.allocate());
        if let (Some(result), Some(result_type)) = (result, result_type) {
            self.remember_call_result(result, result_type, &key, method);
        }
        instructions.push(BackendInstruction::Call {
            callee,
            arguments: lowered_arguments,
            result,
            span: *span,
        });
        result.ok_or_else(|| unsupported(*span, "unit performance method cannot produce a value"))
    }

    fn method_callee(
        &self,
        key: &(String, String),
        method: &str,
        span: SourceSpan,
    ) -> Result<SymbolIdentity, BackendContractError> {
        self.performance_symbols
            .get(key)
            .cloned()
            .or_else(|| {
                let candidates = self
                    .performance_symbols
                    .iter()
                    .filter(|((_, candidate), _)| candidate == method)
                    .map(|(_, symbol)| symbol)
                    .collect::<Vec<_>>();
                (candidates.len() == 1).then(|| candidates[0].clone())
            })
            .or_else(|| {
                self.return_types
                    .contains_key(method)
                    .then(|| calls::callee_for(self, method, span).ok())
                    .flatten()
            })
            .ok_or_else(|| unsupported(span, "method has no static symbol"))
    }

    fn is_binding_receiver(&self, expression: &Expr) -> bool {
        match expression {
            Expr::Identifier { name, .. } => self.binding_value(name).is_some(),
            Expr::FieldAccess { object, .. } | Expr::Index { target: object, .. } => {
                self.is_binding_receiver(object)
            }
            _ => false,
        }
    }

    fn scalar_loan_parameter_address(&self, expression: &Expr) -> Option<BackendValueId> {
        let Expr::Identifier { name, .. } = expression else { return None };
        self.scalar_loan_parameters
            .iter()
            .find(|(parameter, _, _, _)| parameter == name)
            .map(|(_, address, _, _)| *address)
    }

    fn method_result_type(&self, key: &(String, String), method: &str) -> Option<BackendValueType> {
        self.performance_return_types
            .get(key)
            .copied()
            .flatten()
            .or_else(|| {
                let candidates = self
                    .performance_return_types
                    .iter()
                    .filter(|((_, candidate), _)| candidate == method)
                    .filter_map(|(_, ty)| *ty)
                    .collect::<Vec<_>>();
                (candidates.len() == 1).then(|| candidates[0])
            })
            .or_else(|| self.return_types.get(method).copied().flatten())
    }

    fn remember_call_result(
        &mut self,
        result: BackendValueId,
        result_type: BackendValueType,
        key: &(String, String),
        method: &str,
    ) {
        self.remember_value(result, result_type);
        let nominal_type = self
            .performance_return_nominal_types
            .get(key)
            .and_then(Option::as_ref)
            .or_else(|| self.return_nominal_types.get(method).and_then(Option::as_ref));
        if let Some(nominal_type) = nominal_type {
            self.remember_nominal_type(result, nominal_type.clone());
        }
        if let Some(payload_types) = self.return_payload_types.get(method) {
            for (discriminant, payload_type) in payload_types.iter().enumerate() {
                if let Some(payload_type) = payload_type {
                    self.remember_enum_payload(result, discriminant as u32, *payload_type);
                }
            }
        }
    }

    pub(super) fn lower_unary(
        &mut self,
        expression: &Expr,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
        let Expr::Unary { operator, expression, span } = expression else {
            return Err(unsupported(expression_span(expression), "unary expression expected"));
        };
        let operand = self.lower_expression(expression, instructions)?;
        let operation = match operator {
            UnaryOp::Negate => BackendIntegerUnaryOp::Negate,
            UnaryOp::LogicalNot => BackendIntegerUnaryOp::LogicalNot,
            UnaryOp::BitwiseNot => BackendIntegerUnaryOp::BitwiseNot,
        };
        let result = self.allocate();
        let result_type = if matches!(operator, UnaryOp::LogicalNot) {
            BackendValueType::Bool
        } else {
            self.value_type(operand)
                .ok_or_else(|| unsupported(*span, "integer unary operand type is unavailable"))?
        };
        self.remember_value(result, result_type);
        instructions.push(BackendInstruction::IntegerUnary {
            result,
            operation,
            operand,
            ty: result_type,
            span: *span,
        });
        Ok(result)
    }

    pub(super) fn lower_binary(
        &mut self,
        expression: &Expr,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
        let Expr::Binary { left, operator, right, span } = expression else {
            return Err(unsupported(expression_span(expression), "binary expression expected"));
        };
        let left = self.lower_expression(left, instructions)?;
        let right = self.lower_expression(right, instructions)?;
        let operation = match operator {
            BinaryOp::Add => BackendIntegerBinaryOp::Add,
            BinaryOp::Subtract => BackendIntegerBinaryOp::Subtract,
            BinaryOp::Multiply => BackendIntegerBinaryOp::Multiply,
            BinaryOp::Divide => BackendIntegerBinaryOp::Divide,
            BinaryOp::Remainder => BackendIntegerBinaryOp::Remainder,
            BinaryOp::ShiftLeft => BackendIntegerBinaryOp::ShiftLeft,
            BinaryOp::ShiftRight => BackendIntegerBinaryOp::ShiftRight,
            BinaryOp::BitwiseAnd => BackendIntegerBinaryOp::BitwiseAnd,
            BinaryOp::BitwiseOr => BackendIntegerBinaryOp::BitwiseOr,
            BinaryOp::BitwiseXor => BackendIntegerBinaryOp::BitwiseXor,
            BinaryOp::LogicalAnd => BackendIntegerBinaryOp::LogicalAnd,
            BinaryOp::LogicalOr => BackendIntegerBinaryOp::LogicalOr,
            BinaryOp::LessThan
            | BinaryOp::LessEquals
            | BinaryOp::GreaterThan
            | BinaryOp::GreaterEquals
            | BinaryOp::Equals
            | BinaryOp::NotEquals => {
                return self.lower_compare(left, right, *operator, *span, instructions);
            }
        };
        self.lower_integer_binary(operation, left, right, *span, instructions)
    }
}
