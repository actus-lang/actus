use crate::ast::{BinaryOp, CaseMode, Expr, UnaryOp, VariantPayload, VerbDecl};
use crate::lexer::SourceSpan;
use crate::semantic::LoopExitKind;
use std::collections::BTreeMap;
mod aggregate;
mod arithmetic;
mod case;
mod cast;
mod cleanup;
mod drop;
mod layout;
mod literals;
use super::super::{
    BackendCleanupOrigin, BackendContractError, BackendExpressionProvider, BackendInstruction,
    BackendIntegerBinaryOp, BackendIntegerCompareOp, BackendIntegerUnaryOp, BackendSymbolBindings,
    BackendValueId, BackendValueType, SymbolIdentity, SymbolKind,
};
use super::support::{backend_value_type, expression_span, parse_integer, unsupported};
pub(super) struct SharedExpressionProvider<'a> {
    namespace: &'a str,
    layouts: &'a dyn super::super::BackendLayoutProvider,
    parameters: BTreeMap<String, BackendValueId>,
    locals: Vec<BTreeMap<String, BackendValueId>>,
    return_types: &'a BTreeMap<String, Option<BackendValueType>>,
    return_payload_types: &'a BTreeMap<String, [Option<BackendValueType>; 2]>,
    external_symbols: &'a BTreeMap<String, SymbolIdentity>,
    bindings: Option<&'a BackendSymbolBindings>,
    cleanup_plans: &'a [super::super::BackendCleanupPlan],
    value_types: BTreeMap<BackendValueId, BackendValueType>,
    nominal_types: BTreeMap<BackendValueId, crate::ast::TypeName>,
    enum_payload_types: BTreeMap<(BackendValueId, u32), BackendValueType>,
    next_value: u32,
}
impl<'a> SharedExpressionProvider<'a> {
    pub(super) fn new(
        namespace: &'a str,
        verb: &VerbDecl,
        layouts: &'a impl super::super::BackendLayoutProvider,
        return_types: &'a BTreeMap<String, Option<BackendValueType>>,
        return_payload_types: &'a BTreeMap<String, [Option<BackendValueType>; 2]>,
        external_symbols: &'a BTreeMap<String, SymbolIdentity>,
        bindings: Option<&'a BackendSymbolBindings>,
        cleanup_plans: &'a [super::super::BackendCleanupPlan],
    ) -> Self {
        let parameter_bindings = verb
            .params
            .iter()
            .enumerate()
            .map(|(index, parameter)| (parameter.name.clone(), BackendValueId(index as u32)))
            .collect::<BTreeMap<_, _>>();
        let parameter_types = verb
            .params
            .iter()
            .enumerate()
            .filter_map(|(index, parameter)| {
                super::super::signature::lower_parameter(parameter, layouts)
                    .ok()
                    .map(|parameter| (BackendValueId(index as u32), parameter.ty))
            })
            .collect::<BTreeMap<_, _>>();
        let nominal_types = aggregate::parameter_nominal_types(verb, layouts);
        let enum_payload_types = verb
            .params
            .iter()
            .enumerate()
            .filter(|(_, parameter)| {
                parameter.ty.name == "Result" && parameter.ty.arguments.len() == 2
            })
            .flat_map(|(index, parameter)| {
                parameter.ty.arguments.iter().enumerate().filter_map(
                    move |(discriminant, payload)| {
                        backend_value_type(layouts, payload).map(|payload_type| {
                            ((BackendValueId(index as u32), discriminant as u32), payload_type)
                        })
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        Self {
            namespace,
            layouts,
            parameters: parameter_bindings,
            locals: Vec::new(),
            return_types,
            return_payload_types,
            external_symbols,
            bindings,
            cleanup_plans,
            value_types: parameter_types,
            nominal_types,
            enum_payload_types,
            next_value: verb.params.len() as u32,
        }
    }
    pub(super) fn allocate(&mut self) -> BackendValueId {
        let result = BackendValueId(self.next_value);
        self.next_value += 1;
        result
    }
    pub(super) fn parameter_value(&self, name: &str) -> Option<BackendValueId> {
        self.parameters.get(name).copied()
    }
    pub(super) fn value_type(&self, value: BackendValueId) -> Option<BackendValueType> {
        self.value_types.get(&value).copied()
    }
    pub(super) fn remember_value(&mut self, value: BackendValueId, ty: BackendValueType) {
        self.value_types.insert(value, ty);
    }
    pub(super) fn nominal_type(&self, value: BackendValueId) -> Option<&crate::ast::TypeName> {
        self.nominal_types.get(&value)
    }
    pub(super) fn remember_nominal_type(
        &mut self,
        value: BackendValueId,
        type_name: crate::ast::TypeName,
    ) {
        self.nominal_types.insert(value, type_name);
    }
    pub(super) fn remember_enum_payload(
        &mut self,
        value: BackendValueId,
        discriminant: u32,
        ty: BackendValueType,
    ) {
        self.enum_payload_types.insert((value, discriminant), ty);
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
        let skip_scope_cleanup = matches!(
            instructions.last(),
            Some(BackendInstruction::Jump { span, .. }) if self.is_loop_exit(*span)
        );
        if !skip_scope_cleanup {
            cleanup::emit_scope_cleanup(self, span, instructions);
        }
        self.locals.pop();
        Ok(())
    }

    fn distribute_scope_cleanup(
        &mut self,
        span: SourceSpan,
        blocks: &mut Vec<super::super::BackendBlock>,
        start: usize,
    ) -> Result<(), BackendContractError> {
        for block in blocks.iter_mut().skip(start) {
            if matches!(
                block.instructions.last(),
                Some(BackendInstruction::Return { .. })
                    | Some(BackendInstruction::Fail { .. })
                    | Some(BackendInstruction::Jump { .. })
            ) {
                cleanup::emit_scope_cleanup(self, span, &mut block.instructions);
            }
        }
        Ok(())
    }

    fn is_loop_exit(&self, span: SourceSpan) -> bool {
        self.cleanup_plans.iter().any(|plan| {
            matches!(
                plan.origin,
                BackendCleanupOrigin::Loop { span: boundary, .. } if boundary == span
            )
        })
    }

    fn emit_loop_cleanup(
        &mut self,
        span: SourceSpan,
        kind: LoopExitKind,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<(), BackendContractError> {
        cleanup::append_loop_cleanup(self, span, kind, instructions);
        Ok(())
    }

    fn drop_binding(
        &mut self,
        name: &str,
        span: SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<(), BackendContractError> {
        drop::lower_binding(self, name, span, instructions)
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
        self.lower_integer_binary_impl(operation, left, right, span, instructions)
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
            Expr::Integer { value, suffix, span } => {
                let value = parse_integer(value).ok_or_else(|| {
                    unsupported(*span, format!("invalid scalar integer literal `{value}`"))
                })?;
                let ty = literals::integer_literal_type(suffix.as_deref());
                let result = self.allocate();
                instructions.push(BackendInstruction::Constant { result, ty, value, span: *span });
                self.remember_value(result, ty);
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
                self.remember_value(result, BackendValueType::Bool);
                Ok(result)
            }
            Expr::BufferLiteral { length, span } => {
                literals::lower_buffer_literal(self, length, *span, instructions)
            }
            Expr::Grouping { expression, .. } => self.lower_expression(expression, instructions),
            Expr::Cast { expression, target, span } => {
                if matches!(expression.as_ref(), Expr::Integer { .. }) {
                    self.lower_constant_cast(expression, target, *span, instructions)
                } else {
                    self.lower_runtime_cast(expression, target, *span, instructions)
                }
            }
            Expr::Unary { .. } => self.lower_unary(expression, instructions),
            Expr::Binary { .. } => self.lower_binary(expression, instructions),
            Expr::Call { .. } => self.lower_call(expression, instructions),
            Expr::MethodCall { .. } => self.lower_method_call(expression, instructions),
            Expr::StructLit { .. } | Expr::FieldAccess { .. } => {
                aggregate::lower_aggregate_expression(self, expression, instructions)
            }
            _ => Err(unsupported(
                expression_span(expression),
                "expression is outside the scalar shared lowering contract",
            )),
        }
    }

    fn lower_enum_variant_condition_value(
        &mut self,
        subject: BackendValueId,
        variant: &str,
        span: SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
        case::lower_enum_variant_condition_value(self, subject, variant, span, instructions)
    }

    fn bind_enum_variant_payload(
        &mut self,
        subject: BackendValueId,
        variant: &str,
        payload: Option<&VariantPayload>,
        mode: CaseMode,
        span: SourceSpan,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<(), BackendContractError> {
        case::bind_enum_variant_payload(self, subject, variant, payload, mode, span, instructions)
    }

    fn callee_for(
        &mut self,
        callee: &str,
        span: SourceSpan,
    ) -> Result<SymbolIdentity, BackendContractError> {
        self.external_symbols
            .get(callee)
            .cloned()
            .or_else(|| {
                self.return_types
                    .contains_key(callee)
                    .then(|| SymbolIdentity::new(self.namespace, SymbolKind::Verb, callee))
                    .and_then(Result::ok)
            })
            .or_else(|| {
                self.bindings
                    .map(|bindings| bindings.external_symbol(self.namespace, callee))
                    .filter(|symbol| *symbol != callee)
                    .and_then(|symbol| SymbolIdentity::external_raw(symbol).ok())
            })
            .map_or_else(|| SymbolIdentity::new(self.namespace, SymbolKind::Verb, callee), Ok)
            .map_err(|error| unsupported(span, error.to_string()))
    }
}

impl SharedExpressionProvider<'_> {
    fn lower_method_call(
        &mut self,
        expression: &Expr,
        instructions: &mut Vec<BackendInstruction>,
    ) -> Result<BackendValueId, BackendContractError> {
        super::constructors::lower_method_call(self, expression, instructions)
    }

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
        self.remember_value(result, BackendValueType::Integer { signed: true, width: 32 });
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
        self.remember_value(result, BackendValueType::Bool);
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
        if let Some(value) = layout::lower_intrinsic(self, callee, *span, instructions)? {
            return Ok(value);
        }
        let arguments = arguments
            .iter()
            .map(|argument| self.lower_expression(&argument.expression, instructions))
            .collect::<Result<Vec<_>, _>>()?;
        let result_type = self.return_types.get(callee).copied().flatten();
        let result = result_type.map(|_| self.allocate());
        if let (Some(result), Some(result_type)) = (result, result_type) {
            self.remember_value(result, result_type);
            if let Some(payload_types) = self.return_payload_types.get(callee) {
                for (discriminant, payload_type) in payload_types.iter().enumerate() {
                    if let Some(payload_type) = payload_type {
                        self.remember_enum_payload(result, discriminant as u32, *payload_type);
                    }
                }
            }
        }
        instructions.push(BackendInstruction::Call {
            callee: self.callee_for(callee, *span)?,
            arguments,
            result,
            span: *span,
        });
        result.ok_or_else(|| unsupported(*span, "unit call cannot produce a value"))
    }
}
