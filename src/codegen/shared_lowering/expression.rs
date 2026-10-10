use std::collections::BTreeMap;

use crate::ast::{BinaryOp, Expr, UnaryOp, VerbDecl};
use crate::lexer::SourceSpan;

use super::super::{
    BackendContractError, BackendExpressionProvider, BackendInstruction, BackendIntegerBinaryOp,
    BackendIntegerCompareOp, BackendIntegerUnaryOp, BackendValueId, BackendValueType,
    SymbolIdentity, SymbolKind,
};
use super::support::{expression_span, parse_integer, unsupported};

pub(super) struct SharedExpressionProvider<'a> {
    namespace: &'a str,
    parameters: BTreeMap<String, BackendValueId>,
    locals: Vec<BTreeMap<String, BackendValueId>>,
    return_types: &'a BTreeMap<String, Option<BackendValueType>>,
    external_symbols: &'a BTreeMap<String, SymbolIdentity>,
    cleanup_plans: &'a [super::super::BackendCleanupPlan],
    next_value: u32,
}

impl<'a> SharedExpressionProvider<'a> {
    pub(super) fn new(
        namespace: &'a str,
        verb: &VerbDecl,
        return_types: &'a BTreeMap<String, Option<BackendValueType>>,
        external_symbols: &'a BTreeMap<String, SymbolIdentity>,
        cleanup_plans: &'a [super::super::BackendCleanupPlan],
    ) -> Self {
        let parameter_bindings = verb
            .params
            .iter()
            .enumerate()
            .map(|(index, parameter)| (parameter.name.clone(), BackendValueId(index as u32)))
            .collect::<BTreeMap<_, _>>();
        Self {
            namespace,
            parameters: parameter_bindings,
            locals: Vec::new(),
            return_types,
            external_symbols,
            cleanup_plans,
            next_value: verb.params.len() as u32,
        }
    }

    fn allocate(&mut self) -> BackendValueId {
        let result = BackendValueId(self.next_value);
        self.next_value += 1;
        result
    }

    pub(super) fn parameter_value(&self, name: &str) -> Option<BackendValueId> {
        self.parameters.get(name).copied()
    }

    fn emit_scope_cleanup(&self, span: SourceSpan, instructions: &mut Vec<BackendInstruction>) {
        let returned_value = match instructions.last() {
            Some(BackendInstruction::Return { value, .. }) => *value,
            _ => None,
        };
        let mut cleanup = Vec::new();
        for plan in self.cleanup_plans.iter().filter(|plan| plan.span == span) {
            for (index, action) in plan.actions.iter().enumerate() {
                let super::super::BackendCleanupAction::DropBinding { name, .. } = action else {
                    continue;
                };
                let Some(value) = self.local_value(name) else { continue };
                if returned_value == Some(value) {
                    continue;
                }
                let Some(descriptor) =
                    plan.resolved_bridges.get(index).and_then(|items| items.first())
                else {
                    continue;
                };
                cleanup.push(BackendInstruction::Cleanup {
                    action: action.clone(),
                    operand: Some(descriptor.operand(value)),
                    span: plan.span,
                });
            }
        }
        let terminates = matches!(
            instructions.last(),
            Some(BackendInstruction::Return { .. })
                | Some(BackendInstruction::Fail { .. })
                | Some(BackendInstruction::Jump { .. })
        );
        let insertion = instructions.len().saturating_sub(if terminates { 1 } else { 0 });
        instructions.splice(insertion..insertion, cleanup);
    }

    fn local_value(&self, name: &str) -> Option<BackendValueId> {
        self.locals.iter().rev().find_map(|scope| scope.get(name).copied())
    }
}

impl BackendExpressionProvider for SharedExpressionProvider<'_> {
    fn enter_scope(&mut self, _span: SourceSpan) {
        self.locals.push(BTreeMap::new());
    }

    fn exit_scope(
        &mut self,
        span: SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<(), BackendContractError> {
        self.emit_scope_cleanup(span, instructions);
        self.locals.pop();
        Ok(())
    }

    fn bind_owner(
        &mut self,
        name: &str,
        value: BackendValueId,
        span: SourceSpan,
    ) -> Result<(), BackendContractError> {
        let Some(scope) = self.locals.last_mut() else {
            return Err(unsupported(span, "scalar owner declaration has no active lexical scope"));
        };
        scope.insert(name.to_owned(), value);
        Ok(())
    }

    fn assign_owner(
        &mut self,
        name: &str,
        value: BackendValueId,
        span: SourceSpan,
    ) -> Result<(), BackendContractError> {
        let Some(scope) = self.locals.last_mut() else {
            return Err(unsupported(span, "scalar assignment has no active lexical scope"));
        };
        if scope.contains_key(name) {
            scope.insert(name.to_owned(), value);
            return Ok(());
        }
        if self.locals.len() == 1 && self.parameters.contains_key(name) {
            self.parameters.insert(name.to_owned(), value);
            return Ok(());
        }
        Err(unsupported(
            span,
            "scalar assignment requires a local owner in the current lexical scope",
        ))
    }

    fn lower_integer_binary(
        &mut self,
        operation: BackendIntegerBinaryOp,
        left: BackendValueId,
        right: BackendValueId,
        span: SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
        if matches!(operation, BackendIntegerBinaryOp::Divide | BackendIntegerBinaryOp::Remainder) {
            instructions.push(BackendInstruction::TrapIfZero {
                value: right,
                reason: "integer division or remainder by zero".to_owned(),
                span,
            });
        }
        if matches!(operation, BackendIntegerBinaryOp::Divide) {
            instructions.push(BackendInstruction::TrapIfSignedDivisionOverflow {
                left,
                right,
                width: 32,
                reason: "signed integer division overflow".to_owned(),
                span,
            });
        }
        if matches!(
            operation,
            BackendIntegerBinaryOp::ShiftLeft | BackendIntegerBinaryOp::ShiftRight
        ) {
            instructions.push(BackendInstruction::TrapIfShiftCountOutOfRange {
                count: right,
                width: 32,
                reason: "integer shift count exceeds operand width".to_owned(),
                span,
            });
        }
        let result = self.allocate();
        instructions.push(BackendInstruction::IntegerBinary {
            result,
            operation,
            left,
            right,
            ty: BackendValueType::Integer { signed: true, width: 32 },
            span,
        });
        Ok(result)
    }

    fn lower_expression(
        &mut self,
        expression: &Expr,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
        match expression {
            Expr::Identifier { name, span } => self
                .locals
                .iter()
                .rev()
                .find_map(|scope| scope.get(name).copied())
                .or_else(|| self.parameters.get(name).copied())
                .ok_or_else(|| unsupported(*span, format!("unknown scalar binding `{name}`"))),
            Expr::Integer { value, span, .. } => {
                let value = parse_integer(value).ok_or_else(|| {
                    unsupported(*span, format!("invalid scalar integer literal `{value}`"))
                })?;
                let result = self.allocate();
                instructions.push(BackendInstruction::Constant {
                    result,
                    ty: BackendValueType::Integer { signed: true, width: 32 },
                    value,
                    span: *span,
                });
                Ok(result)
            }
            Expr::BoolLiteral { value, span } => {
                let result = self.allocate();
                instructions.push(BackendInstruction::Constant {
                    result,
                    ty: BackendValueType::Bool,
                    value: u64::from(*value),
                    span: *span,
                });
                Ok(result)
            }
            Expr::Grouping { expression, .. } => self.lower_expression(expression, instructions),
            Expr::Unary { .. } => self.lower_unary(expression, instructions),
            Expr::Binary { .. } => self.lower_binary(expression, instructions),
            Expr::Call { .. } => self.lower_call(expression, instructions),
            _ => Err(unsupported(
                expression_span(expression),
                "expression is outside the scalar shared lowering contract",
            )),
        }
    }

    fn callee_for(
        &mut self,
        callee: &str,
        span: SourceSpan,
    ) -> Result<SymbolIdentity, BackendContractError> {
        self.external_symbols
            .get(callee)
            .cloned()
            .map_or_else(|| SymbolIdentity::new(self.namespace, SymbolKind::Verb, callee), Ok)
            .map_err(|error| unsupported(span, error.to_string()))
    }
}

impl SharedExpressionProvider<'_> {
    fn lower_unary(
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
        instructions.push(BackendInstruction::IntegerUnary {
            result,
            operation,
            operand,
            ty: BackendValueType::Integer { signed: true, width: 32 },
            span: *span,
        });
        Ok(result)
    }

    fn lower_binary(
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
            BinaryOp::LessThan
            | BinaryOp::LessEquals
            | BinaryOp::GreaterThan
            | BinaryOp::GreaterEquals
            | BinaryOp::Equals
            | BinaryOp::NotEquals => {
                return self.lower_compare(left, right, *operator, *span, instructions);
            }
            _ => {
                return Err(unsupported(
                    *span,
                    "binary operator is outside the initial scalar lowering contract",
                ));
            }
        };
        self.lower_integer_binary(operation, left, right, *span, instructions)
    }

    fn lower_compare(
        &mut self,
        left: BackendValueId,
        right: BackendValueId,
        operator: BinaryOp,
        span: SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
        let operation = match operator {
            BinaryOp::LessThan => BackendIntegerCompareOp::LessThan,
            BinaryOp::LessEquals => BackendIntegerCompareOp::LessEquals,
            BinaryOp::GreaterThan => BackendIntegerCompareOp::GreaterThan,
            BinaryOp::GreaterEquals => BackendIntegerCompareOp::GreaterEquals,
            BinaryOp::Equals => BackendIntegerCompareOp::Equals,
            BinaryOp::NotEquals => BackendIntegerCompareOp::NotEquals,
            _ => return Err(unsupported(span, "comparison operator expected")),
        };
        let result = self.allocate();
        instructions.push(BackendInstruction::IntegerCompare {
            result,
            operation,
            left,
            right,
            signed: true,
            span,
        });
        Ok(result)
    }

    fn lower_call(
        &mut self,
        expression: &Expr,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
        let Expr::Call { callee, arguments, span } = expression else {
            return Err(unsupported(expression_span(expression), "call expression expected"));
        };
        let arguments = arguments
            .iter()
            .map(|argument| self.lower_expression(&argument.expression, instructions))
            .collect::<Result<Vec<_>, _>>()?;
        let result_type = self.return_types.get(callee).copied().flatten();
        let result = result_type.map(|_| self.allocate());
        instructions.push(BackendInstruction::Call {
            callee: self.callee_for(callee, *span)?,
            arguments,
            result,
            span: *span,
        });
        result.ok_or_else(|| unsupported(*span, "unit call cannot produce a value"))
    }
}
